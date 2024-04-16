use async_trait::async_trait;

use crate::{commons::exception::connect_exception::ConnectException, domain::filter::data_base_query::DataBaseQuery};

#[async_trait]
pub trait IDBRepository: Clone + Send + Sync {
    //TODO: Replace bytes vector returns with specific entities.
    async fn status(&self) -> Result<(), ConnectException>;
    async fn list_data_bases(&self) -> Result<Vec<String>, ConnectException>;
    async fn list_collections(&self, query: DataBaseQuery) -> Result<Vec<String>, ConnectException>;
    fn info(&self) -> Vec<u8>;
    async fn find(&self, query: DataBaseQuery) -> Result<Option<String>, ConnectException>;
    async fn find_query_lite(&self, query: DataBaseQuery) -> Result<Vec<String>, ConnectException>;
    async fn find_query(&self, query: DataBaseQuery) -> Result<Vec<String>, ConnectException>;
    async fn find_all_lite(&self, query: DataBaseQuery) -> Result<Vec<String>, ConnectException>;
    async fn find_all(&self, query: DataBaseQuery) -> Result<Vec<String>, ConnectException>;
    async fn insert(&self, query: DataBaseQuery, value: String) -> Result<String,ConnectException>;
    fn update(&self, query: DataBaseQuery, value: String) -> Vec<u8>;
    async fn delete(&self, query: DataBaseQuery) -> Result<Vec<String>,ConnectException>;
}