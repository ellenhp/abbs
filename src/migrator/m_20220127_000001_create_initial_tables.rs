use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m_20220127_000001_create_initial_tables"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    // Define how to apply this migration: Create the Bakery table.
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(User::Table)
                    .col(
                        ColumnDef::new(User::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(User::Handle)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(User::Status).string())
                    .col(ColumnDef::new(User::Contact).string())
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(PublicKey::Table)
                    .col(
                        ColumnDef::new(PublicKey::Id)
                            .integer()
                            .auto_increment()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(PublicKey::UserId).integer().not_null())
                    .col(
                        ColumnDef::new(PublicKey::Fingerprint)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .foreign_key(
                        ForeignKeyCreateStatement::new()
                            .from(PublicKey::Table, PublicKey::UserId)
                            .to(User::Table, User::Id),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(Forum::Table)
                    .col(
                        ColumnDef::new(Forum::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Forum::Name).string().not_null())
                    .col(ColumnDef::new(Forum::Description).string())
                    .col(ColumnDef::new(Forum::Parent).integer())
                    .foreign_key(
                        ForeignKeyCreateStatement::new()
                            .from(Forum::Table, Forum::Parent)
                            .to(Forum::Table, Forum::Id),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(Thread::Table)
                    .col(
                        ColumnDef::new(Thread::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Thread::Name).string().not_null())
                    .col(ColumnDef::new(Thread::Created).date_time())
                    .col(ColumnDef::new(Thread::Locked).date_time())
                    .col(
                        ColumnDef::new(Thread::Sticky)
                            .boolean()
                            .default(false)
                            .not_null(),
                    )
                    .col(ColumnDef::new(Thread::Forum).integer().not_null())
                    .col(ColumnDef::new(Thread::Author).integer().not_null())
                    .foreign_key(
                        ForeignKeyCreateStatement::new()
                            .from(Thread::Table, Thread::Forum)
                            .to(Forum::Table, Forum::Id),
                    )
                    .foreign_key(
                        ForeignKeyCreateStatement::new()
                            .from(Thread::Table, Thread::Author)
                            .to(User::Table, User::Id),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(Post::Table)
                    .col(
                        ColumnDef::new(Post::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Post::Created).date_time())
                    .col(ColumnDef::new(Post::Modified).date_time())
                    .col(ColumnDef::new(Post::Author).integer().not_null())
                    .col(ColumnDef::new(Post::Thread).integer().not_null())
                    .foreign_key(
                        ForeignKeyCreateStatement::new()
                            .from(Post::Table, Post::Author)
                            .to(User::Table, User::Id),
                    )
                    .foreign_key(
                        ForeignKeyCreateStatement::new()
                            .from(Post::Table, Post::Thread)
                            .to(Thread::Table, Thread::Id),
                    )
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Post::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Thread::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Forum::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(User::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(PublicKey::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(Iden)]
pub enum Forum {
    Table,
    Id,
    Name,
    Description,
    Parent,
}

#[derive(Iden)]
pub enum Thread {
    Table,
    Id,
    Forum,
    Name,
    Locked,
    Created,
    Author,
    Sticky,
}

#[derive(Iden)]
pub enum Post {
    Table,
    Id,
    Author,
    Created,
    Modified,
    Thread,
}

#[derive(Iden)]
pub enum User {
    Table,
    Id,
    Handle,
    Status,
    Contact,
}

#[derive(Iden)]
pub enum PublicKey {
    Table,
    Id,
    Fingerprint,
    UserId,
}
