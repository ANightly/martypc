/*
    MartyPC
    https://github.com/dbalsom/martypc

    Copyright 2022-2025 Daniel Balsom

    Permission is hereby granted, free of charge, to any person obtaining a
    copy of this software and associated documentation files (the “Software”),
    to deal in the Software without restriction, including without limitation
    the rights to use, copy, modify, merge, publish, distribute, sublicense,
    and/or sell copies of the Software, and to permit persons to whom the
    Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in
    all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
    FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

    --------------------------------------------------------------------------

    frontend_common::resource_manager::path_manager.rs

    File and path services for frontends. File operations are abstracted
    to support both local and web filesystems (for wasm compilation).

    Eventually archive support will be added as well.

*/

use crate::resource_manager::ResourceManager;
use anyhow::Error;
use serde::Deserialize;
use std::{collections::HashMap, path::PathBuf};

const BASEDIR_TOKEN: &'static str = "$basedir$";

#[derive(Clone, Debug, Deserialize)]
pub struct PathConfigItem {
    pub resource: String,
    pub path: String,
    #[serde(default)]
    pub create: bool,
    #[serde(default)]
    pub recurse: bool,
}

pub struct PathManager {
    base_path: PathBuf,
    paths: HashMap<String, Vec<PathBuf>>,
}

impl PathManager {
    pub fn new(base_path: PathBuf) -> Self {
        Self {
            base_path,
            paths: HashMap::new(),
        }
    }

    pub fn add_path(&mut self, resource_name: &str, path_str: &str, create: bool) -> Result<(), Error> {
        let resolved_path = self.resolve_path_internal(path_str)?;

        // Attempt to create directories if they don't exist and their `create` flag is set.
        // Inapplicable on web builds.
        #[cfg(not(target_arch = "wasm32"))]
        if !ResourceManager::path_is_dir(&resolved_path) {
            if create {
                ResourceManager::create_path(&resolved_path)?;
            }
            else {
                return Err(anyhow::anyhow!(format!(
                    "Failed to find resource path for '{resource_name}'!\n\
                    Configured path does not exist or is not a directory: {}",
                    resolved_path.to_str().unwrap_or_default()
                )));
            }
        }

        self.paths
            .entry(resource_name.to_string())
            .and_modify(|e| {
                e.push(resolved_path.clone());
            })
            .or_insert(vec![resolved_path.clone()]);
        Ok(())
    }

    fn resolve_path_internal(&self, in_path: &str) -> Result<PathBuf, Error> {
        let parts: Vec<&str> = in_path.split(BASEDIR_TOKEN).collect();
        if parts.len() > 2 {
            return Err(anyhow::anyhow!(
                "Replacement token should only occur at start: {}",
                in_path
            ));
        }

        if parts.len() == 1 {
            // No symbol was found, just return the path
            Ok(PathBuf::from(in_path))
        }
        else {
            //log::debug!("basedir token found. basedir is: {:?}", self.base_path);
            let resolved_path_str = in_path.replace(BASEDIR_TOKEN, self.base_path.to_str().unwrap());
            /*
            let mut built_path = PathBuf::new();
            built_path.push(&self.base_path);
            built_path.push(PathBuf::from(parts[1]));
             */
            let built_path = PathBuf::from(resolved_path_str);
            //log::debug!("built path: {:?}", built_path);
            Ok(built_path)
        }
    }

    pub fn resource_path(&self, resource_name: &str) -> Option<PathBuf> {
        self.paths.get(resource_name).map(|p| p[0].clone())
    }

    pub fn get_resource_paths(&self, resource_name: &str) -> Option<Vec<PathBuf>> {
        self.paths.get(resource_name).map(|p| p.clone())
    }

    pub fn get_base_path(&self) -> PathBuf {
        self.base_path.clone()
    }

    pub fn create_paths(&self) -> Result<(), Error> {
        for (_, paths) in self.paths.iter() {
            for path in paths.iter() {
                if !ResourceManager::path_exists(path) {
                    ResourceManager::create_path(path)?;
                }
            }
        }
        Ok(())
    }

    pub fn dump_paths(&self) -> Vec<PathBuf> {
        self.paths
            .values()
            .map(|p| p.iter().map(|pi| pi.clone()).collect::<Vec<PathBuf>>())
            .flatten()
            .collect()
    }
}
