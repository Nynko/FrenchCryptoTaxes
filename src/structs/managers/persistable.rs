use serde::Serialize;
use std::fs::{self, File};

use rmp_serde::Serializer;
use serde::de::DeserializeOwned;

use crate::errors::IoError;
use crate::utils::{create_directories_if_needed, file_exists};


/* This trait allow us to persist data by serializing and deserializing files */
pub trait Persistable: Serialize + DeserializeOwned {
    const PATH: &'static str;

    fn new(path:Option<String>)-> Result<Self, IoError>
    where
        Self: Sized
    {
        return Self::_new(path,true);
    }

    fn new_non_persistent()-> Result<Self, IoError>
    where
        Self: Sized
    {
        return Self::_new(None,false);
    }

    fn _new(path: Option<String>, persist: bool) -> Result<Self, IoError>
    where
        Self: Sized,
    {
        let path = path.unwrap_or(Self::PATH.to_string());
        if !file_exists(&path) {
            return Ok(Self::default_new(path,persist));
        } else {
            let file = File::open(&path).map_err(|e| IoError::new(e.to_string()))?;
            let deserialized: Self = rmp_serde::from_read(file).map_err(|e| IoError::new(e.to_string()))?;
            Ok(deserialized)
        }
    }

    fn save(&self) -> Result<(), IoError> {
        let path = self.get_path();
        create_directories_if_needed(path);
        let file = File::create(path).map_err(|e| IoError::new(e.to_string()))?;
        let mut writer = Serializer::new(file);
        self.serialize(&mut writer).map_err(|e| IoError::new(e.to_string()))?;
        Ok(())
    }

    fn delete(&self) -> Result<(), IoError> {
        let path = self.get_path();
        if file_exists(path) {
            fs::remove_file(path).map_err(|e| IoError::new(e.to_string()))?;
        }
        Ok(())
    }

    /* Get the saved path or return the default */
    fn get_path(&self) -> &str;

    /* default value (new value): for instance Vec::new() */
    fn default_new(path: String, persist: bool) -> Self;

    /* Decide if it has to persist or not. This can be used in drop trait */
    fn is_persistent(&self) -> bool;
}