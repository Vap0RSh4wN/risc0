// Copyright 2024 RISC Zero, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::path::PathBuf;

use anyhow::{Context, Result};
use bonsai_sdk::alpha::Client;
use clap::Parser;
use risc0_build::BuildStatus;

use crate::commands::build_guest;
use crate::utils;

/// `cargo risczero deploy`
///
/// NOTE: Requires Docker to be installed and running.
#[derive(Parser)]
pub struct DeployCommand {
    /// Location of the Cargo.toml for the guest code.
    ///
    /// This path is relative to the current directory.
    #[arg(long)]
    pub manifest_path: PathBuf,

    /// Feature flags passed to cargo.
    #[arg(long, value_delimiter = ',')]
    pub features: Vec<String>,

    #[command(flatten)]
    client_envs: utils::ClientEnvs,
}

impl DeployCommand {
    pub fn run(&self) -> Result<()> {
        // Instantiate client first to check for errors
        let client = utils::get_client(&self.client_envs)?;
        self.deploy(client)?;

        Ok(())
    }

    fn deploy(&self, client: Client) -> Result<()> {
        // Ensure we have an up to date artifact before deploying
        if let BuildStatus::Skipped = build_guest::build(&self.manifest_path, &self.features)? {
            eprintln!("Build skipped, nothing to deploy.");
            return Ok(());
        }

        let src_dir = std::env::current_dir().unwrap();
        let root_pkg = risc0_build::get_root_pkg(&self.manifest_path, &src_dir)?;
        for target in risc0_build::get_targets(&root_pkg) {
            if target.is_bin() {
                let elf_path = risc0_build::get_elf_path(&src_dir, &root_pkg.name, &target.name);
                let elf = std::fs::read(&elf_path).with_context(|| {
                    format!("Failed to read ELF file at path: {}", elf_path.display())
                })?;
                let image_id = risc0_binfmt::compute_image_id(&elf)?;
                let image_id_hex = hex::encode(image_id);
                client.upload_img(&image_id_hex, elf)?;
                eprintln!(
                    "Uploaded ELF `{}` with image ID `{}`.",
                    target.name, image_id_hex
                );
            }
        }

        Ok(())
    }
}
