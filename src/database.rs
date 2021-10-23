/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */
use crate::api_error::ApiError;
use serde::{de::DeserializeOwned, Serialize};
use sqlite::{Connection, Value};
use std::{marker::PhantomData, path::PathBuf};

pub struct Database<T: Serialize + DeserializeOwned> {
    pub path: PathBuf,
    pub plugin_id: String,
    _config: PhantomData<T>,
}

impl<T: Serialize + DeserializeOwned> Database<T> {
    pub fn new(mut path: PathBuf, plugin_id: String) -> Self {
        path.push("db.sqlite3");

        Self {
            path,
            plugin_id,
            _config: PhantomData,
        }
    }

    pub fn load_config(&self) -> Result<Option<T>, ApiError> {
        let json = self.load_string()?;

        match json {
            Some(json) => serde_json::from_str(json.as_str()).map_err(ApiError::Serialization),
            None => Ok(None),
        }
    }

    pub fn load_string(&self) -> Result<Option<String>, ApiError> {
        let key = self.key();
        let connection = self.open()?;

        let mut cursor = connection
            .prepare("SELECT value FROM settings WHERE key = ?")
            .map_err(ApiError::Database)?
            .into_cursor();

        cursor
            .bind(&[Value::String(key)])
            .map_err(ApiError::Database)?;

        let row = cursor.next().map_err(ApiError::Database)?;

        let s = row.and_then(|row| row[0].as_string().map(|str| str.to_owned()));

        log::trace!("Loaded settings string {:?}", s);

        Ok(s)
    }

    pub fn save_config(&self, t: &T) -> Result<(), ApiError> {
        let json = serde_json::to_string(t).map_err(ApiError::Serialization)?;
        self.save_string(json)?;
        Ok(())
    }

    pub fn save_string(&self, s: String) -> Result<(), ApiError> {
        log::trace!("Saving settings string {}", s);
        let key = self.key();
        let connection = self.open()?;

        let mut statement = connection
            .prepare("INSERT OR REPLACE INTO settings (key, value) VALUES (?, ?)")
            .map_err(ApiError::Database)?;

        statement
            .bind(1, key.as_str())
            .map_err(ApiError::Database)?;
        statement.bind(2, s.as_str()).map_err(ApiError::Database)?;
        statement.next().map_err(ApiError::Database)?;

        Ok(())
    }

    fn open(&self) -> Result<Connection, ApiError> {
        log::trace!("Opening database {:?}", self.path);
        sqlite::open(self.path.as_path()).map_err(ApiError::Database)
    }

    fn key(&self) -> String {
        format!("addons.config.{}", self.plugin_id)
    }
}
