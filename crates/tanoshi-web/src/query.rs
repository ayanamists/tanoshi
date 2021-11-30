use graphql_client::GraphQLQuery;
use std::error::Error;
use wasm_bindgen::prelude::*;

type NaiveDateTime = String;

use crate::{
    common::Cover,
    utils::{graphql_host, local_storage},
};

async fn post_graphql<Q>(var: Q::Variables) -> Result<Q::ResponseData, Box<dyn std::error::Error>>
where
    Q: GraphQLQuery,
{
    let url = graphql_host();

    let token = local_storage()
        .get("token")
        .unwrap_throw()
        .unwrap_or_else(|| "".to_string());
    let request_body = Q::build_query(var);

    let client = reqwest::Client::new();

    let mut req = client.post(url);
    if !token.is_empty() {
        req = req.header("Authorization", format!("Bearer {}", token));
    }
    let res = req.json(&request_body).send().await?;
    let response_body: graphql_client::Response<Q::ResponseData> = res.json().await?;
    match (response_body.data, response_body.errors) {
        (Some(data), _) => Ok(data) as Result<_, _>,
        (_, Some(errors)) => {
            return Err(errors
                .iter()
                .map(|e| format!("{}", e))
                .collect::<Vec<String>>()
                .join(", ")
                .into());
        }
        _ => Err("no data".into()),
    }
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/browse_source.graphql",
    response_derives = "Debug, Clone, PartialEq, Eq"
)]
pub struct BrowseSource;

pub async fn fetch_manga_from_source(
    source_id: i64,
    page: i64,
    keyword: Option<String>,
    sort_by: browse_source::SortByParam,
    sort_order: browse_source::SortOrderParam,
) -> Result<Vec<Cover>, Box<dyn Error>> {
    let var = browse_source::Variables {
        source_id: Some(source_id),
        keyword,
        page: Some(page),
        sort_by: Some(sort_by),
        sort_order: Some(sort_order),
    };
    let data: browse_source::ResponseData = post_graphql::<BrowseSource>(var).await?;

    let covers = data
        .browse_source
        .iter()
        .map(|item| {
            Cover::new(
                item.id,
                source_id,
                item.path.clone(),
                item.title.clone(),
                item.cover_url.clone(),
                item.is_favorite,
                None,
                0,
            )
        })
        .collect();
    Ok(covers)
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/browse_favorites.graphql",
    response_derives = "Debug"
)]
pub struct BrowseFavorites;

pub async fn fetch_manga_from_favorite(
    refresh: bool,
    category_id: Option<i64>,
) -> Result<Vec<Cover>, Box<dyn Error>> {
    let var = browse_favorites::Variables {
        refresh: Some(refresh),
        category_id,
    };
    let data = post_graphql::<BrowseFavorites>(var).await?;

    Ok(data
        .library
        .iter()
        .map(|item| {
            Cover::new(
                item.id,
                0,
                item.path.clone(),
                item.title.clone(),
                item.cover_url.clone(),
                false,
                item.last_read_at.as_ref().and_then(|read_at| {
                    chrono::NaiveDateTime::parse_from_str(read_at, "%Y-%m-%dT%H:%M:%S%.f").ok()
                }),
                item.unread_chapter_count,
            )
        })
        .collect())
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/fetch_manga_by_source_path.graphql",
    response_derives = "Debug"
)]
pub struct FetchMangaBySourcePath;

pub async fn fetch_manga_by_source_path(
    source_id: i64,
    path: String,
) -> Result<fetch_manga_by_source_path::FetchMangaBySourcePathMangaBySourcePath, Box<dyn Error>> {
    let var = fetch_manga_by_source_path::Variables {
        source_id: Some(source_id),
        path: Some(path),
    };
    let data = post_graphql::<FetchMangaBySourcePath>(var).await?;

    Ok(data.manga_by_source_path)
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/fetch_manga_detail.graphql",
    response_derives = "Debug"
)]
pub struct FetchMangaDetail;

pub async fn fetch_manga_detail(
    id: i64,
    refresh: bool,
) -> Result<fetch_manga_detail::FetchMangaDetailManga, Box<dyn Error>> {
    let var = fetch_manga_detail::Variables {
        id: Some(id),
        refresh: Some(refresh),
    };
    let data = post_graphql::<FetchMangaDetail>(var).await?;

    Ok(data.manga)
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/fetch_chapter.graphql",
    response_derives = "Debug"
)]
pub struct FetchChapter;

pub async fn fetch_chapter(
    chapter_id: i64,
) -> Result<fetch_chapter::FetchChapterChapter, Box<dyn Error>> {
    let var = fetch_chapter::Variables {
        chapter_id: Some(chapter_id),
    };
    let data = post_graphql::<FetchChapter>(var).await?;

    Ok(data.chapter)
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/add_to_library.graphql",
    response_derives = "Debug"
)]
pub struct AddToLibrary;

pub async fn add_to_library(manga_id: i64, category_ids: Vec<i64>) -> Result<(), Box<dyn Error>> {
    let var = add_to_library::Variables {
        manga_id: Some(manga_id),
        category_ids: Some(category_ids.iter().map(|id| Some(*id)).collect()),
    };
    let _ = post_graphql::<AddToLibrary>(var).await?;

    Ok(())
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/delete_from_library.graphql",
    response_derives = "Debug"
)]
pub struct DeleteFromLibrary;

pub async fn delete_from_library(manga_id: i64) -> Result<(), Box<dyn Error>> {
    let var = delete_from_library::Variables {
        manga_id: Some(manga_id),
    };
    let _ = post_graphql::<DeleteFromLibrary>(var).await?;

    Ok(())
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/fetch_category_detail.graphql",
    response_derives = "Debug"
)]
pub struct FetchCategoryDetail;

pub async fn fetch_category_detail(id: i64) -> Result<fetch_category_detail::FetchCategoryDetailGetCategory, Box<dyn Error>> {
    let var = fetch_category_detail::Variables {
        id: Some(id)
    };
    let data = post_graphql::<FetchCategoryDetail>(var).await?;

    Ok(data.get_category)
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/fetch_categories.graphql",
    response_derives = "Debug"
)]
pub struct FetchCategories;

pub async fn fetch_categories() -> Result<Vec<fetch_categories::FetchCategoriesGetCategories>, Box<dyn Error>> {
    let var = fetch_categories::Variables {};
    let data = post_graphql::<FetchCategories>(var).await?;

    Ok(data.get_categories)
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/create_category.graphql",
    response_derives = "Debug"
)]
pub struct CreateCategory;

pub async fn create_category(name: &str) -> Result<(), Box<dyn Error>> {
    let var = create_category::Variables {
        name: Some(name.to_string()),
    };
    let _ = post_graphql::<CreateCategory>(var).await?;

    Ok(())
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/update_category.graphql",
    response_derives = "Debug"
)]
pub struct UpdateCategory;

pub async fn update_category(id: i64, name: &str) -> Result<(), Box<dyn Error>> {
    let var = update_category::Variables {
        id: Some(id),
        name: Some(name.to_string()),
    };
    let _ = post_graphql::<UpdateCategory>(var).await?;

    Ok(())
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/delete_category.graphql",
    response_derives = "Debug"
)]
pub struct DeleteCategory;

pub async fn delete_category(id: i64) -> Result<(), Box<dyn Error>> {
    let var = delete_category::Variables { id: Some(id) };
    let _ = post_graphql::<DeleteCategory>(var).await?;

    Ok(())
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/update_page_read_at.graphql",
    response_derives = "Debug"
)]
pub struct UpdatePageReadAt;

pub async fn update_page_read_at(chapter_id: i64, page: i64) -> Result<(), Box<dyn Error>> {
    let var = update_page_read_at::Variables {
        chapter_id: Some(chapter_id),
        page: Some(page),
    };
    let _ = post_graphql::<UpdatePageReadAt>(var).await?;

    Ok(())
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/fetch_recent_updates.graphql",
    response_derives = "Debug"
)]
pub struct FetchRecentUpdates;

pub async fn fetch_recent_updates(
    cursor: Option<String>,
) -> Result<fetch_recent_updates::FetchRecentUpdatesRecentUpdates, Box<dyn Error>> {
    let var = fetch_recent_updates::Variables {
        first: Some(20),
        cursor,
    };
    let data = post_graphql::<FetchRecentUpdates>(var).await?;
    Ok(data.recent_updates)
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/fetch_histories.graphql",
    response_derives = "Debug"
)]
pub struct FetchHistories;

pub async fn fetch_histories(
    cursor: Option<String>,
) -> Result<fetch_histories::FetchHistoriesRecentChapters, Box<dyn Error>> {
    let var = fetch_histories::Variables {
        first: Some(20),
        cursor,
    };
    let data = post_graphql::<FetchHistories>(var).await?;
    Ok(data.recent_chapters)
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/fetch_sources.graphql",
    response_derives = "Debug"
)]
pub struct FetchSources;

pub async fn fetch_sources(
) -> Result<std::vec::Vec<fetch_sources::FetchSourcesInstalledSources>, Box<dyn Error>> {
    let var = fetch_sources::Variables {};
    let data = post_graphql::<FetchSources>(var).await?;
    Ok(data.installed_sources)
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/fetch_all_sources.graphql",
    response_derives = "Debug"
)]
pub struct FetchAllSources;

pub async fn fetch_all_sources() -> Result<fetch_all_sources::ResponseData, Box<dyn Error>> {
    let var = fetch_all_sources::Variables {};
    let data = post_graphql::<FetchAllSources>(var).await?;
    Ok(data)
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/fetch_source.graphql",
    response_derives = "Debug"
)]
pub struct FetchSourceDetail;

pub async fn fetch_source(
    source_id: i64,
) -> Result<fetch_source_detail::FetchSourceDetailSource, Box<dyn Error>> {
    let var = fetch_source_detail::Variables {
        source_id: Some(source_id),
    };
    let data = post_graphql::<FetchSourceDetail>(var).await?;
    Ok(data.source)
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/install_source.graphql",
    response_derives = "Debug"
)]
pub struct InstallSource;

pub async fn install_source(source_id: i64) -> Result<i64, Box<dyn Error>> {
    let var = install_source::Variables {
        source_id: Some(source_id),
    };
    let data = post_graphql::<InstallSource>(var).await?;
    Ok(data.install_source)
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/update_source.graphql",
    response_derives = "Debug"
)]
pub struct UpdateSource;

pub async fn update_source(source_id: i64) -> Result<i64, Box<dyn Error>> {
    let var = update_source::Variables {
        source_id: Some(source_id),
    };
    let data = post_graphql::<UpdateSource>(var).await?;
    Ok(data.update_source)
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/uninstall_source.graphql",
    response_derives = "Debug"
)]
pub struct UninstallSource;

pub async fn uninstall_source(source_id: i64) -> Result<i64, Box<dyn Error>> {
    let var = uninstall_source::Variables {
        source_id: Some(source_id),
    };
    let data = post_graphql::<UninstallSource>(var).await?;
    Ok(data.uninstall_source)
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/login.graphql",
    response_derives = "Debug"
)]
pub struct UserLogin;

pub async fn user_login(username: String, password: String) -> Result<String, Box<dyn Error>> {
    let var = user_login::Variables {
        username: Some(username),
        password: Some(password),
    };
    let data = post_graphql::<UserLogin>(var).await?;
    Ok(data.login)
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/fetch_users.graphql",
    response_derives = "Debug"
)]
pub struct FetchUserList;

pub async fn fetch_users() -> Result<
    (
        fetch_user_list::FetchUserListMe,
        Vec<fetch_user_list::FetchUserListUsers>,
    ),
    Box<dyn Error>,
> {
    let var = fetch_user_list::Variables {};
    let data = post_graphql::<FetchUserList>(var).await?;
    Ok((data.me, data.users))
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/fetch_me.graphql",
    response_derives = "Debug"
)]
pub struct FetchMe;

pub async fn fetch_me() -> Result<fetch_me::FetchMeMe, Box<dyn Error>> {
    let var = fetch_me::Variables {};
    let data = post_graphql::<FetchMe>(var).await?;
    Ok(data.me)
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/register.graphql",
    response_derives = "Debug"
)]
pub struct UserRegister;

pub async fn user_register(
    username: String,
    password: String,
    is_admin: bool,
) -> Result<(), Box<dyn Error>> {
    let var = user_register::Variables {
        username: Some(username),
        password: Some(password),
        is_admin: Some(is_admin),
    };
    let _ = post_graphql::<UserRegister>(var).await?;
    Ok(())
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/change_password.graphql",
    response_derives = "Debug"
)]
pub struct ChangeUserPassword;

pub async fn change_password(
    old_password: String,
    new_password: String,
) -> Result<(), Box<dyn Error>> {
    let var = change_user_password::Variables {
        old_password: Some(old_password),
        new_password: Some(new_password),
    };
    let _ = post_graphql::<ChangeUserPassword>(var).await?;
    Ok(())
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/update_profile.graphql",
    response_derives = "Debug"
)]
pub struct UpdateProfile;

pub async fn update_profile(
    telegram_chat_id: Option<i64>,
    pushover_user_key: Option<String>,
) -> Result<(), Box<dyn Error>> {
    let var = update_profile::Variables {
        input: update_profile::ProfileInput {
            telegramChatId: telegram_chat_id,
            pushoverUserKey: pushover_user_key,
        },
    };
    let _ = post_graphql::<UpdateProfile>(var).await?;
    Ok(())
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/fetch_server_status.graphql",
    response_derives = "Debug"
)]
pub struct FetchServerStatus;

pub async fn server_status(
) -> Result<fetch_server_status::FetchServerStatusServerStatus, Box<dyn Error>> {
    let var = fetch_server_status::Variables {};
    let data = post_graphql::<FetchServerStatus>(var).await?;
    Ok(data.server_status)
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/test_telegram.graphql",
    response_derives = "Debug"
)]
pub struct TestTelegram;

pub async fn test_telegram(chat_id: i64) -> Result<(), Box<dyn Error>> {
    let var = test_telegram::Variables {
        chat_id: Some(chat_id),
    };
    let _ = post_graphql::<TestTelegram>(var).await?;
    Ok(())
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/test_pushover.graphql",
    response_derives = "Debug"
)]
pub struct TestPushover;

pub async fn test_pushover(user_key: &str) -> Result<(), Box<dyn Error>> {
    let var = test_pushover::Variables {
        user_key: Some(user_key.to_string()),
    };
    let _ = post_graphql::<TestPushover>(var).await?;
    Ok(())
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/test_desktop_notification.graphql",
    response_derives = "Debug"
)]
pub struct TestDesktopNotification;

pub async fn test_desktop_notification() -> Result<(), Box<dyn Error>> {
    let var = test_desktop_notification::Variables {};
    let _ = post_graphql::<TestDesktopNotification>(var).await?;
    Ok(())
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/mark_chapter_as_read.graphql",
    response_derives = "Debug"
)]
pub struct MarkChapterAsRead;

pub async fn mark_chapter_as_read(chapter_ids: &[i64]) -> Result<(), Box<dyn Error>> {
    let var = mark_chapter_as_read::Variables {
        chapter_ids: Some(chapter_ids.iter().map(|id| Some(*id)).collect()),
    };

    let _ = post_graphql::<MarkChapterAsRead>(var).await?;
    Ok(())
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/mark_chapter_as_unread.graphql",
    response_derives = "Debug"
)]
pub struct MarkChapterAsUnread;

pub async fn mark_chapter_as_unread(chapter_ids: &[i64]) -> Result<(), Box<dyn Error>> {
    let var = mark_chapter_as_unread::Variables {
        chapter_ids: Some(chapter_ids.iter().map(|id| Some(*id)).collect()),
    };

    let _ = post_graphql::<MarkChapterAsUnread>(var).await?;
    Ok(())
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/download_chapters.graphql",
    response_derives = "Debug"
)]
pub struct DownloadChapters;

pub async fn download_chapters(chapter_ids: &[i64]) -> Result<(), Box<dyn Error>> {
    let var = download_chapters::Variables {
        ids: Some(chapter_ids.iter().map(|id| Some(*id)).collect()),
    };

    let _ = post_graphql::<DownloadChapters>(var).await?;
    Ok(())
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/remove_downloaded_chapters.graphql",
    response_derives = "Debug"
)]
pub struct RemoveDownloadedChapters;

pub async fn remove_downloaded_chapters(chapter_ids: &[i64]) -> Result<(), Box<dyn Error>> {
    let var = remove_downloaded_chapters::Variables {
        ids: Some(chapter_ids.iter().map(|id| Some(*id)).collect()),
    };

    let _ = post_graphql::<RemoveDownloadedChapters>(var).await?;
    Ok(())
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/fetch_download_queue.graphql",
    response_derives = "Debug"
)]
pub struct FetchDownloadQueue;

pub async fn fetch_download_queue(
) -> Result<Vec<fetch_download_queue::FetchDownloadQueueDownloadQueue>, Box<dyn Error>> {
    let var = fetch_download_queue::Variables {};

    Ok(post_graphql::<FetchDownloadQueue>(var)
        .await?
        .download_queue)
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/fetch_downloaded_chapters.graphql",
    response_derives = "Debug"
)]
pub struct FetchDownloadedChapters;

pub async fn fetch_downloaded_chapters(
    cursor: Option<String>,
) -> Result<fetch_downloaded_chapters::FetchDownloadedChaptersGetDownloadedChapters, Box<dyn Error>>
{
    let var = fetch_downloaded_chapters::Variables {
        first: Some(20),
        cursor,
    };
    let data = post_graphql::<FetchDownloadedChapters>(var).await?;
    Ok(data.get_downloaded_chapters)
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/update_chapter_priority.graphql",
    response_derives = "Debug"
)]
pub struct UpdateChapterPriority;

pub async fn update_chapter_priority(chapter_id: i64, priority: i64) -> Result<(), Box<dyn Error>> {
    let var = update_chapter_priority::Variables {
        id: Some(chapter_id),
        priority: Some(priority),
    };
    let _ = post_graphql::<UpdateChapterPriority>(var).await?;
    Ok(())
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/remove_chapter_from_queue.graphql",
    response_derives = "Debug"
)]
pub struct RemoveChapterFromQueue;

pub async fn remove_chapter_from_queue(ids: &[i64]) -> Result<(), Box<dyn Error>> {
    let var = remove_chapter_from_queue::Variables {
        ids: Some(ids.iter().map(|id| Some(*id)).collect()),
    };
    let _ = post_graphql::<RemoveChapterFromQueue>(var).await?;
    Ok(())
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/pause_download.graphql",
    response_derives = "Debug"
)]
pub struct PauseDownload;

pub async fn pause_download() -> Result<bool, Box<dyn Error>> {
    let var = pause_download::Variables {};
    let data = post_graphql::<PauseDownload>(var).await?;
    Ok(data.pause_download)
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/resume_download.graphql",
    response_derives = "Debug"
)]
pub struct ResumeDownload;

pub async fn resume_download() -> Result<bool, Box<dyn Error>> {
    let var = resume_download::Variables {};
    let data = post_graphql::<ResumeDownload>(var).await?;
    Ok(data.resume_download)
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/download_status.graphql",
    response_derives = "Debug"
)]
pub struct DownloadStatus;

pub async fn download_status() -> Result<bool, Box<dyn Error>> {
    let var = download_status::Variables {};
    let data = post_graphql::<DownloadStatus>(var).await?;
    Ok(data.download_status)
}
