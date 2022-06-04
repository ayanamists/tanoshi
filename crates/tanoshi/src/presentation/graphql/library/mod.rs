use super::catalogue::Manga;
use crate::{
    domain::services::{
        chapter::ChapterService, history::HistoryService, library::LibraryService,
        tracker::TrackerService,
    },
    infrastructure::{
        auth::Claims,
        domain::repositories::{
            chapter::ChapterRepositoryImpl, history::HistoryRepositoryImpl,
            library::LibraryRepositoryImpl, tracker::TrackerRepositoryImpl,
        },
        utils::{decode_cursor, encode_cursor},
    },
};
use async_graphql::{
    connection::{query, Connection, Edge, EmptyFields},
    Error,
};
use async_graphql::{Context, Object, Result};
use chrono::{Local, NaiveDateTime};
// use tanoshi_tracker::{anilist, myanimelist, AniList, MyAnimeList, Tracker};

mod categories;
pub use categories::{Category, CategoryMutationRoot, CategoryRoot};

mod recent;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
pub use recent::{RecentChapter, RecentUpdate};

#[derive(Default)]
pub struct LibraryRoot;

#[Object]
impl LibraryRoot {
    async fn library(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "refresh data from source", default = false)] _refresh: bool,
        #[graphql(desc = "category id")] category_id: Option<i64>,
    ) -> Result<Vec<Manga>> {
        let claims = ctx
            .data::<Claims>()
            .map_err(|_| "token not exists, please login")?;

        let manga = ctx
            .data::<LibraryService<LibraryRepositoryImpl>>()?
            .get_manga_from_library_by_category_id(claims.sub, category_id)
            .await?
            .into_par_iter()
            .map(|m| m.into())
            .collect();

        Ok(manga)
    }

    async fn recent_updates(
        &self,
        ctx: &Context<'_>,
        after: Option<String>,
        before: Option<String>,
        first: Option<i32>,
        last: Option<i32>,
    ) -> Result<Connection<String, RecentUpdate, EmptyFields, EmptyFields>> {
        let claims = ctx
            .data::<Claims>()
            .map_err(|_| "token not exists, please login")?;

        let library_svc = ctx.data::<LibraryService<LibraryRepositoryImpl>>()?;

        query(
            after,
            before,
            first,
            last,
            |after, before, first, last| async move {
                let (after_timestamp, after_id) = after
                    .and_then(|cursor: String| decode_cursor(&cursor).ok())
                    .unwrap_or((Local::now().timestamp(), 1));
                let (before_timestamp, before_id) = before
                    .and_then(|cursor: String| decode_cursor(&cursor).ok())
                    .unwrap_or((0, 0));

                let edges = library_svc
                    .get_library_recent_updates(
                        claims.sub,
                        after_timestamp,
                        after_id,
                        before_timestamp,
                        before_id,
                        first,
                        last,
                    )
                    .await?;

                let mut has_previous_page = false;
                if let Some(e) = edges.first() {
                    has_previous_page = library_svc
                        .get_library_recent_updates(
                            claims.sub,
                            Local::now().timestamp(),
                            1,
                            e.uploaded.timestamp(),
                            e.chapter_id,
                            None,
                            Some(1),
                        )
                        .await?
                        .len()
                        > 0;
                }

                let mut has_next_page = false;
                if let Some(e) = edges.last() {
                    has_next_page = library_svc
                        .get_library_recent_updates(
                            claims.sub,
                            e.uploaded.timestamp(),
                            e.chapter_id,
                            0,
                            0,
                            Some(1),
                            None,
                        )
                        .await?
                        .len()
                        > 0;
                }

                let mut connection = Connection::new(has_previous_page, has_next_page);
                connection.append(edges.into_iter().map(|e| {
                    Edge::new(
                        encode_cursor(e.uploaded.timestamp(), e.chapter_id),
                        e.into(),
                    )
                }));

                Ok::<_, Error>(connection)
            },
        )
        .await
    }

    async fn recent_chapters(
        &self,
        ctx: &Context<'_>,
        after: Option<String>,
        before: Option<String>,
        first: Option<i32>,
        last: Option<i32>,
    ) -> Result<Connection<String, RecentChapter, EmptyFields, EmptyFields>> {
        let claims = ctx
            .data::<Claims>()
            .map_err(|_| "token not exists, please login")?;

        let history_svc = ctx.data::<HistoryService<HistoryRepositoryImpl>>()?;

        query(
            after,
            before,
            first,
            last,
            |after, before, first, last| async move {
                let (after_timestamp, _after_id) = after
                    .and_then(|cursor: String| decode_cursor(&cursor).ok())
                    .unwrap_or((Local::now().timestamp(), 1));
                let (before_timestamp, _before_id) = before
                    .and_then(|cursor: String| decode_cursor(&cursor).ok())
                    .unwrap_or((NaiveDateTime::from_timestamp(0, 0).timestamp(), 0));

                let edges = history_svc
                    .get_history_chapters(
                        claims.sub,
                        after_timestamp,
                        before_timestamp,
                        first,
                        last,
                    )
                    .await?;

                let mut has_previous_page = false;
                if let Some(e) = edges.first() {
                    has_previous_page = history_svc
                        .get_history_chapters(
                            claims.sub,
                            Local::now().timestamp(),
                            e.read_at.timestamp(),
                            None,
                            Some(1),
                        )
                        .await?
                        .len()
                        > 0;
                }

                let mut has_next_page = false;
                if let Some(e) = edges.last() {
                    has_next_page = history_svc
                        .get_history_chapters(claims.sub, e.read_at.timestamp(), 0, Some(1), None)
                        .await?
                        .len()
                        > 0;
                }

                let mut connection = Connection::new(has_previous_page, has_next_page);
                connection.append(edges.into_iter().map(|e| {
                    Edge::new(encode_cursor(e.read_at.timestamp(), e.manga_id), e.into())
                }));

                Ok::<_, Error>(connection)
            },
        )
        .await
    }
}

#[derive(Default)]
pub struct LibraryMutationRoot;

#[Object]
impl LibraryMutationRoot {
    async fn add_to_library(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "manga id")] manga_id: i64,
        #[graphql(desc = "category ids")] category_ids: Vec<i64>,
    ) -> Result<u64> {
        let claims = ctx
            .data::<Claims>()
            .map_err(|_| "token not exists, please login")?;

        ctx.data::<LibraryService<LibraryRepositoryImpl>>()?
            .insert_manga_to_library(claims.sub, manga_id, category_ids)
            .await?;

        Ok(1)
    }

    async fn delete_from_library(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "manga id")] manga_id: i64,
    ) -> Result<u64> {
        let claims = ctx
            .data::<Claims>()
            .map_err(|_| "token not exists, please login")?;

        ctx.data::<LibraryService<LibraryRepositoryImpl>>()?
            .delete_manga_from_library(claims.sub, manga_id)
            .await?;

        Ok(1)
    }

    async fn update_page_read_at(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "chapter id")] chapter_id: i64,
        #[graphql(desc = "page")] page: i64,
        #[graphql(desc = "is_complete")] is_complete: bool,
    ) -> Result<u64> {
        let claims = ctx
            .data::<Claims>()
            .map_err(|_| "token not exists, please login")?;

        ctx.data::<HistoryService<HistoryRepositoryImpl>>()?
            .insert_chapter_to_history(claims.sub, chapter_id, page, is_complete)
            .await?;

        let chapter = ctx
            .data::<ChapterService<ChapterRepositoryImpl>>()?
            .fetch_chapter_by_id(chapter_id)
            .await?;

        // TODO: nepnep source have weird number, don't update tracker status for them for now
        if !is_complete || (chapter.source_id == 3 || chapter.source_id == 4) {
            return Ok(1);
        }

        let tracker_svc = ctx.data::<TrackerService<TrackerRepositoryImpl>>()?;

        let tracked_manga = tracker_svc
            .get_tracked_manga_id(claims.sub, chapter.manga_id)
            .await?;

        for manga in tracked_manga {
            if let Some(tracker_manga_id) = manga.tracker_manga_id {
                // TODO: Only update if chapter > then read
                tracker_svc
                    .update_manga_tracking_status(
                        claims.sub,
                        &manga.tracker,
                        tracker_manga_id,
                        None,
                        None,
                        Some(chapter.number as i64),
                        None,
                        None,
                    )
                    .await?;
            }
        }

        Ok(1)
    }

    async fn mark_chapter_as_read(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "chapter ids")] chapter_ids: Vec<i64>,
    ) -> Result<u64> {
        let claims = ctx
            .data::<Claims>()
            .map_err(|_| "token not exists, please login")?;

        ctx.data::<HistoryService<HistoryRepositoryImpl>>()?
            .insert_chapters_to_history_as_completed(claims.sub, chapter_ids)
            .await?;

        Ok(1)
    }

    async fn mark_chapter_as_unread(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "chapter ids")] chapter_ids: Vec<i64>,
    ) -> Result<u64> {
        let claims = ctx
            .data::<Claims>()
            .map_err(|_| "token not exists, please login")?;

        ctx.data::<HistoryService<HistoryRepositoryImpl>>()?
            .delete_chapters_from_history(claims.sub, chapter_ids)
            .await?;

        Ok(1)
    }
}
