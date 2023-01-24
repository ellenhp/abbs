pub mod search;
pub mod viewer;

use std::fs::{create_dir, remove_dir, rename};
use std::path::Path;
use std::sync::{Arc, Mutex};

use tantivy::collector::TopDocs;
use tantivy::query::{BooleanQuery, FuzzyTermQuery, Occur, Query};
use tantivy::schema::STORED;
use tantivy::{
    schema::{Field, Schema, TEXT},
    Index, IndexWriter,
};
use tantivy::{ReloadPolicy, Searcher, Term};
use zim::{DirectoryEntry, MimeType, Target, Zim};

lazy_static! {
    static ref LIBRARY: Mutex<Vec<Library>> = Mutex::new(Vec::new());
}

pub(super) fn push_library(lib: Library) {
    LIBRARY.lock().unwrap().push(lib);
}

pub(super) fn get_library(name: &str) -> Option<Library> {
    LIBRARY
        .lock()
        .unwrap()
        .iter()
        .find(|lib| lib.name == name)
        .cloned()
}

static TITLE_FIELD: &str = "title";
static TITLE_LOWERCASE_FIELD: &str = "title_lowercase";
static CONTENT_FIELD: &str = "content";
static CLUSTER_FIELD: &str = "cluster";
static BLOB_FIELD: &str = "blob";

#[derive(Clone)]
pub(super) struct Library {
    name: String,
    zim: Arc<Zim>,
    searcher: Arc<Searcher>,
    title_field: Field,
    title_lowercase_field: Field,
    content_field: Field,
    cluster_field: Field,
    blob_field: Field,
}

#[derive(Debug, Clone)]
pub(super) struct Article {
    pub(super) title: String,
    pub(super) content_html: String,
}

impl Library {
    pub(super) async fn open<P: AsRef<Path> + Sized>(
        name: &str,
        zim_path: P,
        index_path: P,
    ) -> Result<Self, anyhow::Error> {
        let zim = Zim::new(zim_path)?;
        let index = Self::ensure_indexed(&zim, index_path).await.unwrap();
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommit)
            .try_into()?;

        let searcher = reader.searcher();
        let title_field = index.schema().get_field(TITLE_FIELD).unwrap();
        let title_lowercase_field = index.schema().get_field(TITLE_LOWERCASE_FIELD).unwrap();
        let content_field = index.schema().get_field(CONTENT_FIELD).unwrap();
        let cluster_field = index.schema().get_field("cluster").unwrap();
        let blob_field = index.schema().get_field("blob").unwrap();

        let library = Self {
            name: name.to_string(),
            zim: Arc::new(zim),
            searcher: Arc::new(searcher),
            title_field,
            title_lowercase_field,
            content_field,
            cluster_field,
            blob_field,
        };
        Ok(library)
    }

    async fn ensure_indexed<P: AsRef<Path> + Sized>(
        zim: &Zim,
        index_directory: P,
    ) -> Result<Index, anyhow::Error> {
        let index_directory = index_directory.as_ref();
        if !index_directory.exists() {
            create_dir(&index_directory)?;
        }
        let final_path = index_directory.join(format!("{:X}.idx", zim.checksum).to_string());
        if final_path.exists() {
            return Ok(Index::open_in_dir(final_path)?);
        }

        let tmp_path_string =
            index_directory.join(format!("{:X}.idx.tmp", zim.checksum).to_string());
        let tmp_path: &Path = Path::new(&tmp_path_string);
        if tmp_path.exists() {
            // Only removes the directory if it is empty.
            remove_dir(&tmp_path_string)?;
        }
        create_dir(&tmp_path)?;
        {
            let mut schema_builder = Schema::builder();
            let title = schema_builder.add_text_field(TITLE_FIELD, STORED);
            let title_lowercase = schema_builder.add_text_field(TITLE_LOWERCASE_FIELD, TEXT);
            let cluster = schema_builder.add_u64_field(CLUSTER_FIELD, STORED);
            let blob = schema_builder.add_u64_field(BLOB_FIELD, STORED);
            let content = schema_builder.add_text_field(CONTENT_FIELD, TEXT);
            let schema = schema_builder.build();
            let index = Index::create_in_dir(tmp_path, schema)?;
            let mut index_writer = index.writer(50_000_000)?;
            let mut indexed = 0u32;
            let mut total = 0u32;
            for entry in zim.iterate_by_urls() {
                total += 1;
                if entry.mime_type != MimeType::Type("text/html".to_string()) {
                    continue;
                }

                match Self::index_document(
                    title,
                    title_lowercase,
                    content,
                    cluster,
                    blob,
                    &entry,
                    &zim,
                    &mut index_writer,
                )
                .into()
                {
                    Ok(_) => {
                        indexed += 1;
                    }
                    Err(_) => todo!(),
                };
            }
            index_writer.commit()?;

            println!("Indexed {} of {} entries for zim file", indexed, total);
        }
        rename(tmp_path, final_path.clone())?;
        let index = Index::open_in_dir(final_path)?;

        Ok(index)
    }

    fn index_document(
        title_field: Field,
        title_lowercase_field: Field,
        content_field: Field,
        cluster_field: Field,
        blob_field: Field,
        entry: &DirectoryEntry,
        zim: &Zim,
        index_writer: &mut IndexWriter,
    ) -> Result<(), anyhow::Error> {
        let title = &entry.title;
        let (cluster, blob) = match &entry.target {
            Some(Target::Cluster(cluster, blob)) => (*cluster, *blob),
            _ => return Err(anyhow::anyhow!("Entry has no target")),
        };
        let content_raw = if let Some(Target::Cluster(cluster, blob)) = entry.target {
            let blob =
                String::from_utf8(zim.get_cluster(cluster)?.get_blob(blob)?.as_ref().to_vec())?;
            blob
        } else {
            return Err(anyhow::anyhow!("Entry has no target"));
        };
        let text = html2text::parse(content_raw.as_bytes())
            .render_plain(usize::MAX)
            .into_string()
            .to_lowercase();
        index_writer.add_document(doc!(
            title_field => title.clone(),
            title_lowercase_field => title.clone().to_lowercase(),
            cluster_field => cluster as u64,
            blob_field => blob as u64,
            content_field => text,
        ))?;
        Ok(())
    }

    pub fn search(&self, title: &str, limit: usize) -> Result<Vec<Article>, anyhow::Error> {
        let mut title_queries = Vec::new();
        let mut content_queries = Vec::new();
        {
            let tokens = title.split_whitespace();
            let len = title.split_whitespace().count();
            for (idx, token) in tokens.enumerate() {
                if token.trim().is_empty() {
                    continue;
                }
                let title_term = Term::from_field_text(self.title_lowercase_field, token);
                let content_term = Term::from_field_text(self.content_field, token);
                let (title_query, content_query) = if idx == len - 1 {
                    (
                        FuzzyTermQuery::new_prefix(title_term, 1, true),
                        FuzzyTermQuery::new_prefix(content_term, 1, true),
                    )
                } else {
                    (
                        FuzzyTermQuery::new(title_term, 1, true),
                        FuzzyTermQuery::new(content_term, 1, true),
                    )
                };
                title_queries.push((Occur::Should, Box::new(title_query) as Box<dyn Query>));
                content_queries.push((Occur::Should, Box::new(content_query) as Box<dyn Query>));
            }
        }
        let query = BooleanQuery::new(vec![
            (Occur::Should, Box::new(BooleanQuery::new(title_queries))),
            (Occur::Should, Box::new(BooleanQuery::new(content_queries))),
        ]);
        let top_docs = self.searcher.search(&query, &TopDocs::with_limit(limit))?;
        let mut articles = Vec::new();
        for (_weight, doc) in top_docs {
            let title = self
                .searcher
                .doc(doc)?
                .get_first(self.title_field)
                .map(|t| t.as_text().expect("Title is not text").to_string())
                .unwrap_or("Untitled Document".to_string())
                .to_string();

            let cluster = self
                .searcher
                .doc(doc)?
                .get_first(self.cluster_field)
                .map(|t| t.as_u64().expect("Cluster is not u64") as u32)
                .unwrap_or(0);

            let blob = self
                .searcher
                .doc(doc)?
                .get_first(self.blob_field)
                .map(|t| t.as_u64().expect("Blob is not u64") as u32)
                .unwrap_or(0);

            let content_html = self
                .zim
                .get_cluster(cluster)?
                .get_blob(blob)?
                .as_ref()
                .to_vec();
            articles.push(Article {
                title,
                content_html: String::from_utf8(content_html)?,
            });
        }
        Ok(articles)
    }
}
