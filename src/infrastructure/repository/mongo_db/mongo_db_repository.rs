use async_trait::async_trait;

use mongodb::{
    bson::{doc, to_document, Document},
    options::{AggregateOptions, ClientOptions},
    Client, Collection, Cursor, Database,
};

use futures_util::stream::StreamExt;
use serde_json::{from_str, Value};
use uuid::Uuid;

use crate::{
    commons::{
        configuration::definition::mongo_db::mongo_db,
        exception::connect_exception::ConnectException,
    },
    domain::{
        collection::{
            collection_definition::CollectionDefinition,
            generate_collection_query::GenerateCollectionQuery,
        },
        connection_data::ConnectionData,
        data_base::generate_database_query::GenerateDatabaseQuery,
        document::{
            document_data::DocumentData, document_key::DocumentKey,
            document_key_attribute::DocumentKeyAttribute, document_schema::DocumentSchema,
        },
        e_json_type::EJSONType,
        field::generate::field_data::FieldData,
        filter::{data_base_query::DataBaseQuery, filter_element::FilterElement},
        table::table_data_group::TableDataGroup,
    },
    infrastructure::repository::i_db_repository::IDBRepository,
};

use super::{e_action::EAction, extractor_metadata_mongo_db::ExtractorMetadataMongoDb};

#[derive(Clone)]
pub struct MongoDbRepository {
    client: Client
}

impl MongoDbRepository {
    
    pub async fn new(connection: &ConnectionData) -> Result<impl IDBRepository, ConnectException> {
        let client = MongoDbRepository::connect(connection.connection()).await;
        if client.is_err() {
            let exception = ConnectException::new(client.err().unwrap().to_string());
            return Err(exception);
        }
        
        let instance = MongoDbRepository {
            client: client.ok().unwrap()
        };

        Ok(instance)
    }

    async fn connect(connection: String) -> Result<Client, mongodb::error::Error> {
        let client_options = ClientOptions::parse(connection).await?;
        let client = Client::with_options(client_options)?;

        Ok(client)
    }

        
    fn collection_from_query(&self, query: &DataBaseQuery) -> Collection<Document> {
        let data_base = query.data_base();        
        let collection = query.collection();
        
        self.collection(&data_base, &collection)
    }

    fn collection_from_resource(&self, query: &GenerateCollectionQuery) -> Collection<Document> {
        let data_base = query.data_base();        
        let collection = query.collection();

        self.collection(&data_base, &collection)
    }

    fn data_base(&self, data_base: &String) -> Database {
        self.client.database(&data_base)
    }

    fn collection(&self, data_base: &String, collection: &String) -> Collection<Document> {
        self.data_base(data_base).collection(&collection)
    }

    async fn find_cursor(&self, query: &DataBaseQuery) -> Result<Cursor<Document>, ConnectException>  {
        let collection = self.collection_from_query(&query);

        let mut filter = FilterElement::new();

        let o_filter = query.filter();
        if o_filter.is_some() {
            filter = o_filter.unwrap();
        }

        let pipeline: Result<Vec<Document>, ConnectException> = filter.as_mongo_agregate();
        if pipeline.is_err() {
            return Err(pipeline.err().unwrap());
        }
    
        let r_cursor = collection.aggregate(pipeline.ok().unwrap(), AggregateOptions::default()).await;
        if r_cursor.is_err() {
            let exception = ConnectException::new(r_cursor.unwrap_err().to_string());
            return Err(exception);
        }

        Ok(r_cursor.unwrap())
    }

    async fn collections_metadata_document(&self, data_base: String, collection: String) -> Result<Document, ConnectException> {
        let data_base = self.data_base(&data_base);
        Ok(data_base
            .run_command(doc! {"collStats": collection}, None).await.unwrap())
    }

    fn document_keys(&self, document: &Document) -> Result<Vec<DocumentKey>, ConnectException> {
        let mut keys = Vec::new();

        let key = "_id";

        let o_id = document.get(key);
        if let None = o_id {
            let exception = ConnectException::new(String::from("Identifier not found."));
            return Err(exception);
        }

        let base_key = match document.get_object_id(key) {
            Ok(oid) => DocumentKey::new(
                String::from("_id"), 
                oid.to_hex(), 
                EJSONType::STRING,
                Vec::from(vec![
                    DocumentKeyAttribute::new(String::from("$oid"), String::from("true"))
                ]
            )),
            Err(_) => {
                let id = o_id.unwrap().as_str();
                if let None = id {
                    let exception = ConnectException::new(String::from("Identifier not found."));
                    return Err(exception);
                }
                DocumentKey::new(
                String::from(key), 
                String::from(id.unwrap()),
                EJSONType::STRING,
                Vec::new())
            },
        };
        
        keys.push(base_key);

        Ok(keys)
    }

    async fn query_action(&self, query: &DataBaseQuery, action: EAction, value: Option<&str>) -> Result<Vec<DocumentData>, ConnectException> {
        let mut elements = Vec::<DocumentData>::new();
        
        let collection = self.collection_from_query(&query);

        let mut cursor = self.find_cursor(query).await?;
        while let Some(r_document) = cursor.next().await {
            if r_document.is_err() {
                let exception = ConnectException::new(r_document.unwrap_err().to_string());
                return Err(exception);
            }
    
            let document = r_document.unwrap();
            let data = self.make_document_data(query, &document)?;
            elements.push(data);

            match action {
                EAction::FIND => (),
                EAction::DELETE => self.delete_document(&collection, &document).await?,
                EAction::UPDATE => self.update_document(&collection, &document, value).await?,
            }
        }
     
        Ok(elements)
    }

    fn make_document_data(&self, query: &DataBaseQuery, document: &Document) -> Result<DocumentData, ConnectException> {
        let json = serde_json::to_string(&document);
        if json.is_err() {
            let exception = ConnectException::new(json.unwrap_err().to_string());
            return Err(exception);
        }

        let keys = self.document_keys(&document)?;
        let base_key = keys.iter().find(|k| k.name() == "_id");
        if let None = base_key {
            let exception = ConnectException::new(String::from("Base identifier not found."));
            return Err(exception);
        }

        Ok(DocumentData::new(
            query.data_base(), query.collection(), base_key.cloned(),
            keys, json.ok().unwrap()
        ))
    }

    async fn delete_document(&self, collection: &Collection<Document>, document: &Document) -> Result<(), ConnectException> {
        let result = collection.delete_one(document.clone(), None).await;
        if result.is_err() {
            let exception = ConnectException::new(result.unwrap_err().to_string());
            return Err(exception);
        }
        Ok(())
    }

    async fn update_document(&self, collection: &Collection<Document>, document: &Document, value: Option<&str>) -> Result<(), ConnectException> {
        if let None = value {
            let exception = ConnectException::new(String::from("Cannot update None document."));
            return Err(exception);
        }

        let new_document = self.document_from_string(&value.unwrap())?;

        let result = collection.replace_one(document.clone(), new_document, None).await;
        if result.is_err() {
            let exception = ConnectException::new(result.unwrap_err().to_string());
            return Err(exception);
        }
        Ok(())
    }

    fn document_from_string(&self, value: &str) -> Result<Document, ConnectException> {
        let json: Result<Value, _> = from_str(value);
        if json.is_err() {
            let error_message = format!("Invalid JSON format: {}", json.err().unwrap());
            let exception = ConnectException::new(error_message);
            return Err(exception);
        }

        let document = to_document(&json.unwrap());
        if document.is_err() {
            let err = format!("Failed to convert JSON to BSON: {}", document.unwrap_err());
            return Err(ConnectException::new(err));
        }

        Ok(document.unwrap())
    }

}

#[async_trait]
impl IDBRepository for MongoDbRepository {

    async fn status(&self) -> Result<(), ConnectException> {
        let _ = self.data_base_find_all().await?;
        return Ok(());
    }

    async fn metadata(&self) -> Result<Vec<TableDataGroup>, ConnectException> {
        let server_info = &self.client.database("admin")
            .run_command(doc! {"serverStatus": 1}, None).await.unwrap();

        ExtractorMetadataMongoDb::from_db(server_info)
    }

    async fn data_base_find_all(&self) -> Result<Vec<String>, ConnectException> {
        let result = self.client.list_database_names(None, None).await;
        if result.is_err() {
            let exception = ConnectException::new(result.err().unwrap().to_string());
            return Err(exception);
        }
        
        Ok(result.ok().unwrap())
    }

    async fn data_base_exists(&self, query: &DataBaseQuery) -> Result<bool, ConnectException> {
        let databases = self.data_base_find_all().await?;

        Ok(databases.iter().any(|name| name == &query.data_base()))
    }

    async fn data_base_create(&self, query: &GenerateDatabaseQuery) -> Result<String, ConnectException> {
        let data_base = query.data_base();
        let temp_col = format!("TEMP_{}", Uuid::new_v4().to_string());
        if self.collection_exists(&DataBaseQuery::from(data_base.clone(), temp_col.clone())).await? {
            return self.data_base_create(query).await;
        }

        let query = GenerateCollectionQuery::from_collection(data_base.clone(), temp_col);
        let _ = self.collection_create(&query).await?;

        Ok(data_base)
    }

    async fn data_base_drop(&self, query: &GenerateDatabaseQuery) -> Result<String, ConnectException> {
        let data_base = query.data_base();
        let database = self.data_base(&data_base);
        let result = database.drop(None).await;
        if result.is_err() {
            let exception = ConnectException::new(result.err().unwrap().to_string());
            return Err(exception);
        }

        Ok(data_base)
    }

    async fn data_base_metadata(&self, query: &DataBaseQuery) -> Result<Vec<TableDataGroup>, ConnectException> {
        let mut documents = Vec::new();
        
        let collections = self.collection_find_all(query).await?;
        for collection in collections {
            let document = self.collections_metadata_document(query.data_base(), collection).await?;
            documents.push(document);
        }

        ExtractorMetadataMongoDb::from_collections(documents)
    }

    async fn collection_accept_schema(&self) -> Result<CollectionDefinition, ConnectException> {        
        let json = mongo_db();

        let definition: CollectionDefinition = serde_json::from_str(&json).expect("Failed to parse JSON");

        Ok(definition)
    }

    async fn collection_metadata(&self, query: &DataBaseQuery) -> Result<Vec<TableDataGroup>, ConnectException> {
        let document = self.collections_metadata_document(query.data_base(), query.collection()).await?;

        ExtractorMetadataMongoDb::from_collection(document)
    }

    async fn collection_find_all(&self, query: &DataBaseQuery) -> Result<Vec<String>, ConnectException> {
        let result = self.data_base(&query.data_base()).list_collection_names(None).await;
        if result.is_err() {
            let exception = ConnectException::new(result.unwrap_err().to_string());
            return Err(exception);
        }

        Ok(result.ok().unwrap())
    }

    async fn collection_exists(&self, query: &DataBaseQuery) -> Result<bool, ConnectException> {
        let collections = self.find(query).await?;
        
        Ok(collections.iter().any(|document| &document.collection() == &query.collection()))
    }

    async fn collection_create(&self, query: &GenerateCollectionQuery) -> Result<String, ConnectException> {
        let name = query.collection();
        let db = self.data_base(&query.data_base());
        let result = db.create_collection(&name, None).await;
        if let Err(result) = result {
            let exception = ConnectException::new(result.to_string());
            return Err(exception);
        }

        let collection: Collection<Document> = db.collection(&name);

        if query.fields().len() > 0 {
            let indexes = FieldData::collection_as_mongo_create(query.fields())?;
            if let Err(result) = collection.create_indexes(indexes, None).await {
                let _ = self.collection_drop(query).await?;
                let exception = ConnectException::new(result.to_string());
                return Err(exception);
            }
        }

        Ok(name)
    }

    async fn collection_drop(&self, query: &GenerateCollectionQuery) -> Result<String, ConnectException> {
        let collection = self.collection_from_resource(&query);
        let result = collection.drop(None).await;
        if let Err(error) = result {
            let exception = ConnectException::new(error.to_string());
            return Err(exception);
        }

        Ok(query.collection())
    }

    async fn collection_rename(&self, query: &DataBaseQuery, name: &str) -> Result<String, ConnectException> {
        let admin_db = &self.client.database("admin");
        let command = doc! {
            "renameCollection": format!("{}.{}", query.data_base(), query.collection()),
            "to": format!("{}.{}", query.data_base(), name)
        };

        if let Err(error) = admin_db.run_command(command, None).await {
            let exception = ConnectException::new(error.to_string());
            return Err(exception);
        }

        Ok(String::from(name))
    }

    async fn collection_export(&self, query: &DataBaseQuery) -> Result<Vec<DocumentData>, ConnectException> {
        self.find_all(query).await
    }

    async fn collection_import(&self, query: &DataBaseQuery, documents: Vec<String>) -> Result<String, ConnectException> {
        for document in documents {
            self.insert(query, &document).await?;
        }
        Ok(String::new())
    }

    async fn find_query_lite(&self, query: &DataBaseQuery) -> Result<Vec<String>, ConnectException> {
        let mut elements = Vec::<String>::new();
        
        let mut cursor = self.find_cursor(query).await?;
        while let Some(result) = cursor.next().await {
            if result.is_err() {
                let exception = ConnectException::new(result.unwrap_err().to_string());
                return Err(exception);
            }

            let document = result.ok().unwrap();

            let id = document.get("_id");
            if id.is_some() {
                elements.push(format!("_id={}", id.unwrap().to_string()));
            }
        }

        Ok(elements)
    }

    async fn find_query(&self, query: &DataBaseQuery) -> Result<Vec<DocumentData>, ConnectException> {
        Ok(self.query_action(query, EAction::FIND, None).await?)
    }

    async fn find_all_lite(&self, query: &DataBaseQuery) -> Result<Vec<String>, ConnectException> {
        let fix = DataBaseQuery::from(query.data_base(), query.collection());
        return self.find_query_lite(&fix).await;
    }

    async fn find_all(&self, query: &DataBaseQuery) -> Result<Vec<DocumentData>, ConnectException> {
        let fix = DataBaseQuery::from(query.data_base(), query.collection());
        return self.find_query(&fix).await;
    }

    async fn find(&self, query: &DataBaseQuery) -> Result<Option<DocumentData>, ConnectException> {
        let o_result = self.find_query(query).await;
        if o_result.is_err() {
            return Err(o_result.unwrap_err());
        }
        
        Ok(o_result.unwrap().first().cloned())
    }

    async fn schema(&self, query: &DataBaseQuery) -> Result<DocumentSchema, ConnectException> {
        let fields = Vec::new();
        let comments = Vec::from(vec![
            String::from("If '_id' field is not defined it will be created with an ObjectId default value.")
        ]);
        Ok(DocumentSchema::new(comments, fields))
    }

    async fn insert(&self, query: &DataBaseQuery, value: &str) -> Result<DocumentData, ConnectException> {
        let collection = self.collection_from_query(&query);

        let mut document = self.document_from_string(&value)?;

        let result = collection.insert_one(document.clone(), None).await;
        if result.is_err() {
            let err = format!("Could not insert into database: {}", result.err().unwrap());
            return Err(ConnectException::new(err));
        }

        document.insert("_id", result.unwrap().inserted_id);

        Ok(self.make_document_data(query, &document)?)
    }

    async fn update(&self, query: &DataBaseQuery, value: &str) -> Result<Vec<DocumentData>, ConnectException> {
        Ok(self.query_action(query, EAction::UPDATE, Some(value)).await?)
    }

    async fn delete(&self, query: &DataBaseQuery) -> Result<Vec<DocumentData>, ConnectException> {
        Ok(self.query_action(query, EAction::DELETE, None).await?)
    }
    
}