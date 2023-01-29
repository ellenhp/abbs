use sea_orm_migration::{async_trait, MigrationTrait, MigratorTrait};

mod m_20220127_000001_create_initial_tables;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m_20220127_000001_create_initial_tables::Migration)]
    }
}
