/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

//! Interacting with gateway databases.

use crate::error::WebthingsError;
use serde::{de::DeserializeOwned, Serialize};
use sqlite::{Connection, Value};
use std::{marker::PhantomData, path::PathBuf};

/// A struct which represents a view into a gateway database.
pub struct Database<T: Serialize + DeserializeOwned> {
    /// Location of the database file.
    pub path: PathBuf,
    /// ID of the [plugin][crate::Plugin] associated with this view into the database.
    pub plugin_id: String,
    _config: PhantomData<T>,
}

impl<T: Serialize + DeserializeOwned> Database<T> {
    /// Open an existing gateway database.
    pub fn new(mut path: PathBuf, plugin_id: impl Into<String>) -> Self {
        path.push("db.sqlite3");

        Self {
            path,
            plugin_id: plugin_id.into(),
            _config: PhantomData,
        }
    }

    /// Load config for the associated [plugin][crate::Plugin] from database.
    pub fn load_config(&self) -> Result<Option<T>, WebthingsError> {
        let json = self.load_string()?;

        match json {
            Some(json) => {
                serde_json::from_str(json.as_str()).map_err(WebthingsError::Serialization)
            }
            None => Ok(None),
        }
    }

    /// Load raw string for the associated [plugin][crate::Plugin] from database.
    pub fn load_string(&self) -> Result<Option<String>, WebthingsError> {
        let key = self.key();
        let connection = self.open()?;

        let mut cursor = connection
            .prepare("SELECT value FROM settings WHERE key = ?")
            .map_err(WebthingsError::Database)?
            .into_cursor();

        cursor
            .bind(&[Value::String(key)])
            .map_err(WebthingsError::Database)?;

        let row = cursor.next().map_err(WebthingsError::Database)?;

        let s = row.and_then(|row| row[0].as_string().map(|str| str.to_owned()));

        log::trace!("Loaded settings string {:?}", s);

        Ok(s)
    }

    /// Save config for the associated [plugin][crate::Plugin] to database.
    pub fn save_config(&self, t: &T) -> Result<(), WebthingsError> {
        let json = serde_json::to_string(t).map_err(WebthingsError::Serialization)?;
        self.save_string(json)?;
        Ok(())
    }

    /// Save raw string for the associated [plugin][crate::Plugin] to database.
    pub fn save_string(&self, s: impl Into<String>) -> Result<(), WebthingsError> {
        let s = s.into();
        log::trace!("Saving settings string {}", s);
        let key = self.key();
        let connection = self.open()?;

        let mut statement = connection
            .prepare("INSERT OR REPLACE INTO settings (key, value) VALUES (?, ?)")
            .map_err(WebthingsError::Database)?;

        statement
            .bind(1, key.as_str())
            .map_err(WebthingsError::Database)?;
        statement.bind(2, &*s).map_err(WebthingsError::Database)?;
        statement.next().map_err(WebthingsError::Database)?;

        Ok(())
    }

    fn open(&self) -> Result<Connection, WebthingsError> {
        log::trace!("Opening database {:?}", self.path);
        sqlite::open(self.path.as_path()).map_err(WebthingsError::Database)
    }

    fn key(&self) -> String {
        format!("addons.config.{}", self.plugin_id)
    }
}
