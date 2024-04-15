use std::vec;

use async_trait::async_trait;

use crate::{domain::filter::{data_base_query::DataBaseQuery, filter_element::FilterElement}, infrastructure::repository::i_db_repository::IDBRepository, service::service::Service};

use super::{i_manager::IManager, terminal_cursor::TerminalCursor, terminal_manager::{self, TerminalManager}, terminal_option::TerminalOption};

const HOME: &'static str = "HOME";
const STATUS: &'static str = "STATUS";

const SHOW_DATABASES: &'static str = "SHOW_DATABASES";
const SELECT_DATABASE_PANEL: &'static str = "SELECT_DATABASE_PANEL";
const SELECT_DATABASE: &'static str = "SELECT_DATABASE";

const SHOW_COLLECTIONS: &'static str = "SHOW_COLLECTIONS";
const SELECT_COLLECTION_PANEL: &'static str = "SELECT_COLLECTION_PANEL";
const SELECT_COLLECTION: &'static str = "SELECT_COLLECTION";

const SHOW_ELEMENTS: &'static str = "SHOW_ELEMENTS";
const SELECT_ELEMENTS_PANEL: &'static str = "SELECT_ELEMENTS_PANEL";
const SELECT_ELEMENT: &'static str = "SELECT_ELEMENT";

const SHOW_ELEMENT: &'static str = "SHOW_ELEMENT";

#[derive(Clone)]
pub struct TerminalDatabase<T: IDBRepository> {
    service: Service<T>,
    data_base: Option<String>,
    collection: Option<String>,
    element: Option<String>
}

#[async_trait]
impl <T: IDBRepository> IManager for TerminalDatabase<T> {

    async fn manage(&self, option: TerminalOption<Self>) -> TerminalCursor<Self> where Self: Sized {
        match option.option().as_str() {
            HOME => self.clone().home(&self.default_header()),
            STATUS => self.clone().status().await,

            SHOW_DATABASES => self.clone().show_databases().await,
            SELECT_DATABASE_PANEL => self.clone().select_database_panel().await,
            SELECT_DATABASE => self.clone().select_database(option),

            SHOW_COLLECTIONS => self.clone().show_collections().await,
            SELECT_COLLECTION_PANEL => self.clone().select_collection_panel().await,
            SELECT_COLLECTION => self.clone().select_collection(option),

            SHOW_ELEMENTS => self.clone().show_elements().await,
            SELECT_ELEMENTS_PANEL => self.clone().select_element_panel().await,
            SELECT_ELEMENT => self.clone().select_element(option),

            SHOW_ELEMENT => self.clone().show_element().await,
            _ => todo!(),
        }
    }

}

impl <T: IDBRepository> TerminalDatabase<T> {

    pub fn new(service: Service<T>) -> TerminalDatabase<T> {
        TerminalDatabase { 
            service: service,
            data_base: None,
            collection: None,
            element: None
        }
    }

    pub async fn launch(&mut self) -> &Self {
        let header = self.default_header();
        let cursor = self.home(&header);
        let _ = TerminalManager::new(cursor).launch().await;
        return self;
    }

    pub fn default_header(&self) -> String {
        return self.info_headers("Select any option: ");
    }

    pub fn info_headers(&self, header: &str) -> String {
        let mut headers = Vec::<String>::new();

        if self.data_base.is_some() {
            headers.push(format!("* Selected data base '{}'.", self.data_base.as_ref().unwrap()));
        }

        if self.collection.is_some() {
            headers.push(format!("* Selected collection '{}'.", self.collection.as_ref().unwrap()));
        }

        if self.element.is_some() {
            headers.push(format!("* Selected element '{}'.", self.element.as_ref().unwrap()));
        }

        if headers.is_empty() {
            return String::from(header);
        }

        return format!("{}\n\n{}", header, headers.join("\n"));
    }

    fn home(&self, header: &str) -> TerminalCursor<Self> {
        let mut cursor: TerminalCursor<Self> = TerminalCursor::new(header);

        cursor.push(TerminalOption::from(String::from("Show databases"), SHOW_DATABASES, self.clone()));
        cursor.push(TerminalOption::from(String::from("Select database"), SELECT_DATABASE_PANEL, self.clone()));

        if self.data_base.is_some() {
            cursor.push(TerminalOption::from(String::from("Show collections"), SHOW_COLLECTIONS, self.clone()));
            cursor.push(TerminalOption::from(String::from("Select collection"), SELECT_COLLECTION_PANEL, self.clone()));
        }

        if self.collection.is_some() {
            cursor.push(TerminalOption::from(String::from("Show elements"), SHOW_ELEMENTS, self.clone()));
            cursor.push(TerminalOption::from(String::from("Select element"), SELECT_ELEMENTS_PANEL, self.clone()));
        }

        if self.element.is_some() {
            cursor.push(TerminalOption::from(String::from("Show element"), SHOW_ELEMENT, self.clone()));
        }

        cursor
    }

    async fn status(self) -> TerminalCursor<Self> {
        let cursor = TerminalCursor::new("//TODO:");
        cursor
    }

    async fn show_databases(&self) -> TerminalCursor<Self> {
        let result = self.service.list_data_bases().await;

        let mut header = self.info_headers("The repository contains the following data bases:");
        if let Err(err) = &result {
            header = err.to_string();
        }
    
        let mut vector = Vec::<String>::new();
        if result.is_ok() {
            vector = result.ok().unwrap();
        }

        let mut elements = Vec::<String>::new();
        for element in vector {
            elements.push(format!(" - {}{}{}", terminal_manager::ANSI_BOLD, element, terminal_manager::ANSI_COLOR_RESET));
        }

        if !elements.is_empty() {
            header = format!("{}\n", header);
        }

        self.home(&format!("{}\n{}", header, elements.join("\n")))
    }

    async fn select_database_panel(&self) -> TerminalCursor<Self> {
        let result = self.service.list_data_bases().await;

        let mut header = self.info_headers("Select one of the following data bases:");
        if let Err(err) = &result {
            header = err.to_string();
        }
    
        let mut vector = Vec::<String>::new();
        if result.is_ok() {
            vector = result.ok().unwrap();
        }

        let mut cursor: TerminalCursor<Self> = TerminalCursor::new(&header);

        for element in vector {
            let args = Vec::from(vec![element.clone()]);
            cursor.push(TerminalOption::from_args(element, SELECT_DATABASE, args, self.clone()));
        }

        cursor.push(TerminalOption::from(String::from("[None]"), SELECT_DATABASE, self.clone()));

        cursor
    }


    fn select_database(&mut self, option: TerminalOption<Self>) -> TerminalCursor<Self> {
        let args = option.args();
        if args.len() > 0 {
            let data_base = args.get(0).unwrap().to_string();
            self.data_base = Some(data_base);
        } else {
            self.reset_database();
        }

        self.home(&self.default_header())
    }


    async fn show_collections(&self) -> TerminalCursor<Self> {
        if let Some(error) = self.verify_database() {
            return error;
        }

        let query = DataBaseQuery::from_data_base(self.data_base.clone().unwrap());

        let result = self.service.list_collections(query).await;

        let mut header = self.info_headers("The repository contains the following collections:");
        if let Err(err) = &result {
            header = err.to_string();
        }
    
        let mut vector = Vec::<String>::new();
        if result.is_ok() {
            vector = result.ok().unwrap();
        }

        let mut elements = Vec::<String>::new();
        for element in vector {
            elements.push(format!(" - {}{}{}", terminal_manager::ANSI_BOLD, element, terminal_manager::ANSI_COLOR_RESET));
        }

        if !elements.is_empty() {
            header = format!("{}\n", header);
        }

        self.home(&format!("{}\n{}", header, elements.join("\n")))
    }

    async fn select_collection_panel(&self) -> TerminalCursor<Self> {
        if let Some(error) = self.verify_database() {
            return error;
        }

        let query = DataBaseQuery::from_data_base(self.data_base.clone().unwrap());

        let result = self.service.list_collections(query).await;

        let mut header = self.info_headers("Select one of the following collections:");
        if let Err(err) = &result {
            header = err.to_string();
        }
    
        let mut vector = Vec::<String>::new();
        if result.is_ok() {
            vector = result.ok().unwrap();
        }

        let mut cursor: TerminalCursor<Self> = TerminalCursor::new(&header);

        for element in vector {
            let args = Vec::from(vec![element.clone()]);
            cursor.push(TerminalOption::from_args(element, SELECT_COLLECTION, args, self.clone()));
        }

        cursor.push(TerminalOption::from(String::from("[None]"), SELECT_COLLECTION, self.clone()));

        cursor
    }

    fn select_collection(&mut self, option: TerminalOption<Self>) -> TerminalCursor<Self> {
        let args = option.args();
        if args.len() > 0 {
            let collection = args.get(0).unwrap().to_string();
            self.collection = Some(collection);
        } else {
            self.reset_collection();
        }

        self.home(&self.default_header())
    }


    async fn show_elements(&self) -> TerminalCursor<Self> {
        if let Some(error) = self.verify_collection() {
            return error;
        }

        let query = DataBaseQuery::from(self.data_base.clone().unwrap(), self.collection.clone().unwrap());

        let result = self.service.find_all_lite(query).await;

        let mut header = self.info_headers("The repository contains the following items:");
        if let Err(err) = &result {
            header = err.to_string();
        }
    
        let mut vector = Vec::<String>::new();
        if result.is_ok() {
            vector = result.ok().unwrap();
        }

        let mut elements = Vec::<String>::new();
        for element in vector {
            elements.push(format!(" - {}{}{}", terminal_manager::ANSI_BOLD, element, terminal_manager::ANSI_COLOR_RESET));
        }

        if !elements.is_empty() {
            header = format!("{}\n", header);
        }

        self.home(&format!("{}\n{}", header, elements.join("\n")))
    }

    async fn select_element_panel(&self) -> TerminalCursor<Self> {
        if let Some(error) = self.verify_collection() {
            return error;
        }

        let query = DataBaseQuery::from(self.data_base.clone().unwrap(), self.collection.clone().unwrap());

        let result = self.service.find_all_lite(query).await;

        let mut header = self.info_headers("Select one of the following elements:");
        if let Err(err) = &result {
            header = err.to_string();
        }
    
        let mut vector = Vec::<String>::new();
        if result.is_ok() {
            vector = result.ok().unwrap();
        }

        let mut cursor: TerminalCursor<Self> = TerminalCursor::new(&header);

        for element in vector {
            let args = Vec::from(vec![element.clone()]);
            cursor.push(TerminalOption::from_args(element, SELECT_ELEMENT, args, self.clone()));
        }

        cursor.push(TerminalOption::from(String::from("[None]"), SELECT_ELEMENT, self.clone()));

        cursor
    }

    fn select_element(&mut self, option: TerminalOption<Self>) -> TerminalCursor<Self> {
        let args = option.args();
        if args.len() > 0 {
            let element = args.get(0).unwrap().to_string();
            self.element = Some(element);
        } else {
            self.reset_element();
        }

        self.home(&self.default_header())
    }

    async fn show_element(&self) -> TerminalCursor<Self> {
        if let Some(error) = self.verify_element() {
            return error;
        }

        let filter = FilterElement::from_id_chain(self.element.clone().unwrap());
        let query = DataBaseQuery::from_filter(self.data_base.clone().unwrap(), self.collection.clone().unwrap(), filter);

        let result = self.service.find(query).await;
        if result.is_err() {
            let header = self.info_headers(&format!("Cannot find enlement: {}", result.unwrap_err().to_string()));
            return self.home(&header);
        }

        let document = result.unwrap();
        if document.is_none() {
            let header = self.info_headers("Element not found.");
            return self.home(&header);
        }

        let header = self.info_headers("Item:");
        self.home(&format!("{}\n\n{}", header, document.unwrap()))
    }
    
    
    fn verify_element(&self) -> Option<TerminalCursor<Self>> {
        if self.element.is_none() {
            let header = self.info_headers("No element selected:");
            return Some(self.home(&header));
        }

        self.verify_collection()
    }

    fn verify_collection(&self) -> Option<TerminalCursor<Self>> {
        if self.collection.is_none() {
            let header = self.info_headers("No collection selected:");
            return Some(self.home(&header));
        }

        self.verify_database()
    }

    fn verify_database(&self) -> Option<TerminalCursor<Self>> {
        if self.data_base.is_none() {
            let header = self.info_headers("No data base selected:");
            return Some(self.home(&header));
        }

        None
    }

    fn reset_database(&mut self) {
        self.data_base = None;
        self.reset_collection();
    }

    fn reset_collection(&mut self) {
        self.collection = None;
        self.reset_element();
    }

    fn reset_element(&mut self) {
        self.element = None
    }

}