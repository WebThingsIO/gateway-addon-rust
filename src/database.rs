/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */
use crate::api_error::ApiError;
use serde::{de::DeserializeOwned, Serialize};
use sqlite::{Connection, Value};
use std::path::PathBuf;

pub struct Database {
    path: PathBuf,
    plugin_id: String,
}

impl Database {
    pub fn new(mut path: PathBuf, plugin_id: String) -> Self {
        path.push("db.sqlite3");

        Self { path, plugin_id }
    }

    pub fn load_config<T>(&self) -> Result<Option<T>, ApiError>
    where
        T: DeserializeOwned,
    {
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

    pub fn save_config<T>(&self, t: &T) -> Result<(), ApiError>
    where
        T: Serialize,
    {
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
