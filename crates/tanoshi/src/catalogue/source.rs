use std::{
    collections::{BTreeMap, HashMap},
    str::FromStr,
};

use crate::{guard::AdminGuard, user::Claims};
use async_graphql::{Context, Json, Object, Result, SimpleObject};
use serde::{Deserialize, Serialize};
use tanoshi_lib::prelude::{FilterField, Version};
use tanoshi_vm::prelude::ExtensionBus;

#[derive(Debug, Clone, Deserialize)]
pub struct SourceIndex {
    pub id: i64,
    pub name: String,
    pub path: String,
    pub version: String,
    pub icon: String,
}

impl From<SourceIndex> for Source {
    fn from(index: SourceIndex) -> Self {
        Self {
            id: index.id,
            name: index.name,
            url: "".to_string(),
            version: index.version,
            icon: index.icon,
            need_login: false,
            has_update: false,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, SimpleObject)]
pub struct Filters {
    default: String,
    fields: Json<BTreeMap<String, FilterField>>,
}

impl From<tanoshi_lib::data::Filters> for Filters {
    fn from(v: tanoshi_lib::data::Filters) -> Self {
        Self {
            default: v.default,
            fields: Json(v.fields),
        }
    }
}

#[derive(Clone)]
pub struct Source {
    pub id: i64,
    pub name: String,
    pub url: String,
    pub version: String,
    pub icon: String,
    pub need_login: bool,
    pub has_update: bool,
}

impl From<tanoshi_lib::data::Source> for Source {
    fn from(s: tanoshi_lib::data::Source) -> Self {
        Self {
            id: s.id,
            name: s.name,
            url: s.url,
            version: s.version.to_string(),
            icon: s.icon,
            need_login: s.need_login,
            has_update: false,
        }
    }
}

#[Object]
impl Source {
    async fn id(&self) -> i64 {
        self.id
    }

    async fn name(&self) -> String {
        self.name.clone()
    }

    async fn url(&self) -> String {
        self.url.clone()
    }

    async fn version(&self) -> String {
        self.version.clone()
    }

    async fn icon(&self) -> String {
        self.icon.clone()
    }

    async fn need_login(&self) -> bool {
        self.need_login
    }

    async fn has_update(&self) -> bool {
        self.has_update
    }

    async fn filters(&self, ctx: &Context<'_>) -> Result<Option<Filters>> {
        let extensions = ctx.data::<ExtensionBus>()?;
        if let Some(res) = extensions.filters_async(self.id).await? {
            Ok(Some(res.into()))
        } else {
            Ok(None)
        }
    }
}

#[derive(Default)]
pub struct SourceRoot;

#[Object]
impl SourceRoot {
    async fn installed_sources(
        &self,
        ctx: &Context<'_>,
        check_update: bool,
    ) -> Result<Vec<Source>> {
        let _ = ctx.data::<Claims>()?;
        let installed_sources = ctx.data::<ExtensionBus>()?.list_async().await?;
        let mut sources: Vec<Source> = vec![];
        if check_update {
            let available_sources_map = {
                let url = "https://faldez.github.io/tanoshi-extensions".to_string();
                let available_sources: Vec<SourceIndex> = reqwest::get(&url).await?.json().await?;
                let mut available_sources_map = HashMap::new();
                for source in available_sources {
                    available_sources_map.insert(source.id, source);
                }
                available_sources_map
            };

            for source in installed_sources {
                let mut source: Source = source.into();
                if let Some(index) = available_sources_map.get(&source.id) {
                    source.has_update =
                        Version::from_str(&index.version)? > Version::from_str(&source.version)?;
                }
                sources.push(source);
            }
        } else {
            sources = installed_sources.into_iter().map(|s| s.into()).collect();
        }
        sources.sort_by(|a, b| a.id.cmp(&b.id));

        Ok(sources)
    }

    async fn available_sources(&self, ctx: &Context<'_>) -> Result<Vec<Source>> {
        let _ = ctx.data::<Claims>()?;
        let url = "https://faldez.github.io/tanoshi-extensions".to_string();
        let source_indexes: Vec<SourceIndex> = reqwest::get(&url).await?.json().await?;
        let extensions = ctx.data::<ExtensionBus>()?;

        let mut sources: Vec<Source> = vec![];
        for index in source_indexes {
            if !extensions.exist_async(index.id).await? {
                sources.push(index.into());
            }
        }
        Ok(sources)
    }

    async fn source(&self, ctx: &Context<'_>, source_id: i64) -> Result<Source> {
        let _ = ctx.data::<Claims>()?;
        let exts = ctx.data::<ExtensionBus>()?;
        Ok(exts.detail_async(source_id).await?.into())
    }
}

#[derive(Default)]
pub struct SourceMutationRoot;

#[Object]
impl SourceMutationRoot {
    #[graphql(guard = "AdminGuard::new()")]
    async fn install_source(&self, ctx: &Context<'_>, source_id: i64) -> Result<i64> {
        let extensions = ctx.data::<ExtensionBus>()?;
        if extensions.exist_async(source_id).await? {
            return Err("source installed, use updateSource to update".into());
        }

        let url = "https://faldez.github.io/tanoshi-extensions".to_string();
        let source_indexes: Vec<SourceIndex> = reqwest::get(&url).await?.json().await?;
        let source: SourceIndex = source_indexes
            .iter()
            .find(|index| index.id == source_id)
            .ok_or("source not found")?
            .clone();

        let url = format!(
            "https://faldez.github.io/tanoshi-extensions/library/{}.{}.tanoshi",
            source.name,
            env!("TARGET")
        );

        let raw = reqwest::get(&url).await?.bytes().await?;
        extensions.install_async(source.name, &raw).await?;

        Ok(source.id)
    }

    #[graphql(guard = "AdminGuard::new()")]
    async fn uninstall_source(&self, ctx: &Context<'_>, source_id: i64) -> Result<i64> {
        let extensions = ctx.data::<ExtensionBus>()?;

        extensions.unload_async(source_id).await?;

        Ok(source_id)
    }

    #[graphql(guard = "AdminGuard::new()")]
    async fn update_source(&self, ctx: &Context<'_>, source_id: i64) -> Result<i64> {
        let extensions = ctx.data::<ExtensionBus>()?;
        extensions.exist_async(source_id).await?;

        let url = "https://faldez.github.io/tanoshi-extensions".to_string();

        let source_indexes: Vec<SourceIndex> = reqwest::get(&url).await?.json().await?;
        let source: SourceIndex = source_indexes
            .iter()
            .find(|index| index.id == source_id)
            .ok_or("source not found")?
            .clone();

        if extensions.detail_async(source_id).await?.version == Version::from_str(&source.version)?
        {
            return Err("No new version".into());
        }

        let url = format!(
            "https://faldez.github.io/tanoshi-extensions/library/{}.{}.tanoshi",
            source.name,
            env!("TARGET")
        );

        let raw = reqwest::get(&url).await?.bytes().await?;

        extensions.unload_async(source_id).await?;
        extensions.install_async(source.name, &raw).await?;

        Ok(source_id)
    }
}
