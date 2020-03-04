use super::component::Manga;
use serde::Deserialize;
use yew::format::{Json, Nothing};
use yew::prelude::*;
use yew::services::fetch::{FetchService, FetchTask};
use yew::{html, Component, ComponentLink, Html, Properties, ShouldRender};

use super::component::model::{FavoriteManga, GetFavoritesResponse, GetMangasResponse, MangaModel};
use http::{Request, Response};
use std::borrow::BorrowMut;
use yew::services::storage::Area;
use yew::services::StorageService;
use yew::utils::{document, window};

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

#[derive(Clone, Properties)]
pub struct Props {
    pub source: String,
}

pub struct Catalogue {
    fetch_task: Option<FetchTask>,
    link: ComponentLink<Self>,
    source: String,
    page: i32,
    mangas: Vec<MangaModel>,
    favorites: Vec<String>,
    is_fetching: bool,
    token: String,
}

pub enum Msg {
    MangaReady(GetMangasResponse),
    FavoritesReady(GetFavoritesResponse),
    ScrolledDown,
    Noop,
}

impl Component for Catalogue {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let scroll_callback = link.callback(|_| {
            let current_scroll = window().scroll_y().expect("error get scroll y");
            let height = window().inner_height().unwrap().as_f64().unwrap();
            if current_scroll >= height {
                info!("scroll end");
                return Msg::ScrolledDown;
            }
            Msg::Noop
        });
        let closure = Closure::wrap(Box::new(move || scroll_callback.emit("")) as Box<dyn Fn()>);

        window().set_onscroll(Some(closure.as_ref().unchecked_ref()));
        let storage = StorageService::new(Area::Local).unwrap();
        let token = {
            if let Ok(token) = storage.restore("token") {
                token
            } else {
                "".to_string()
            }
        };
        Catalogue {
            fetch_task: None,
            link,
            source: props.source,
            page: 1,
            mangas: vec![],
            favorites: vec![],
            is_fetching: false,
            token,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::MangaReady(data) => {
                let mut mangas = data.mangas;
                self.mangas.append(&mut mangas);
                self.is_fetching = false;
            }
            Msg::FavoritesReady(data) => {
                self.favorites = data
                    .favorites
                    .unwrap()
                    .iter()
                    .map(|ch| ch.title.clone())
                    .collect();
                self.fetch_mangas();
            }
            Msg::ScrolledDown => {
                if !self.is_fetching {
                    self.page += 1;
                    self.fetch_mangas();
                }
            }

            Msg::Noop => {
                info!("noop");
            }
        }
        true
    }

    fn mounted(&mut self) -> ShouldRender {
        self.fetch_favorites();
        true
    }

    fn view(&self) -> Html {
        html! {
            <div class="container-fluid">
                <div class="row row-cols-sm-2 row-cols-md-3 row-cols-lg-5 row-cols-xl-6">
                { for self.mangas.iter().map(|manga| html!{
                <Manga
                    title=manga.title.to_owned()
                    thumbnail=manga.thumbnail_url.to_owned()
                    path=manga.path.to_owned()
                    source=self.source.to_owned()
                    is_favorite={if self.favorites.contains(&manga.title.to_owned()){true} else {false}}/>
                }) }
                </div>
            </div>
        }
    }
}

impl Catalogue {
    fn fetch_mangas(&mut self) {
        let req = Request::get(format!(
            "/api/source/{}?sort_by=popularity&sort_order=descending&page={}",
            self.source, self.page
        ))
        .body(Nothing)
        .expect("failed to build request");

        if let Ok(task) = FetchService::new().fetch(
            req,
            self.link.callback(
                |response: Response<Json<Result<GetMangasResponse, anyhow::Error>>>| {
                    if let (meta, Json(Ok(data))) = response.into_parts() {
                        if meta.status.is_success() {
                            return Msg::MangaReady(data);
                        }
                    }
                    Msg::Noop
                },
            ),
        ) {
            self.fetch_task = Some(FetchTask::from(task));
            self.is_fetching = true;
        }
    }

    fn fetch_favorites(&mut self) {
        let req = Request::get("/api/favorites")
            .header("Authorization", self.token.clone())
            .body(Nothing)
            .expect("failed to build request");

        if let Ok(task) = FetchService::new().fetch(
            req,
            self.link.callback(
                |response: Response<Json<Result<GetFavoritesResponse, anyhow::Error>>>| {
                    if let (meta, Json(Ok(data))) = response.into_parts() {
                        if meta.status.is_success() {
                            return Msg::FavoritesReady(data);
                        }
                    }
                    Msg::Noop
                },
            ),
        ) {
            self.fetch_task = Some(FetchTask::from(task));
        }
    }
}
