#[allow(dead_code)]
pub(crate) mod aul2_check;
#[allow(dead_code)]
pub(crate) mod aul2_document;
#[allow(dead_code)]
pub(crate) mod aul2_document_builder;
#[allow(dead_code)]
pub(crate) mod aul2_parser;
#[allow(dead_code)]
pub(crate) mod aul2_prune;
#[allow(dead_code)]
pub(crate) mod aul2_serializer;
#[allow(dead_code)]
pub(crate) mod aul2_update;
pub mod build;
pub mod config;
pub mod config_loader;
pub mod configured_script;
pub(crate) mod configured_script_body;
pub(crate) mod configured_script_language;
pub mod direct_script;
pub(crate) mod direct_script_language;
pub mod embed;
pub mod include;
pub mod init;
pub mod install;
pub mod language;
pub mod language_directive;
#[allow(dead_code)]
pub(crate) mod language_file_check;
#[allow(dead_code)]
pub(crate) mod language_file_plan;
#[allow(dead_code)]
pub(crate) mod language_file_request;
#[allow(dead_code)]
pub(crate) mod language_file_update;
#[allow(dead_code)]
pub(crate) mod language_plan;
#[allow(dead_code)]
pub(crate) mod language_script_analysis;
pub mod language_script_entries;
pub mod language_script_info;
#[allow(dead_code)]
pub(crate) mod language_section_catalog;
pub mod language_ui;
pub mod logical_script_language;
pub mod pack;
pub mod schema;
pub mod script_identity;
pub(crate) mod source_processing;
pub mod text_utils;
pub mod ui_control;

#[cfg(test)]
#[path = "../tests/common/mod.rs"]
pub mod common;
