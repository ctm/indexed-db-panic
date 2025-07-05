use {
    indexed_db::{Database, Error, Factory},
    log::{error, info},
    wasm_bindgen::{JsCast, JsValue},
    wasm_bindgen_futures::JsFuture,
    web_sys::{Blob, Event, File, HtmlInputElement, Url},
    yew::{html::Scope, platform::spawn_local, prelude::*},
};

type OurError = ();
type Props = ();

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct ObjectUrl(String);

#[derive(Debug)]
pub(crate) struct Assets {
    pub(super) buttons: Vec<ObjectUrl>,
    pub(super) backgrounds: Vec<ObjectUrl>,
    pub(super) styles: Vec<String>,
}

#[derive(Default)]
pub(crate) struct App {
    db: Option<Database<OurError>>,
}

pub(crate) enum Msg {
    DbBuilt(Database<OurError>),
    StoreAsset(File),
    AssetsRead(Assets),
}

const DB_NAME: &str = "mb";
const INDEX: &str = "file";
const BUTTONS: &str = "buttons";
const BACKGROUNDS: &str = "backgrounds";
const STYLES: &str = "styles";
static FILE_INDEX: [&str; 4] = ["name", "lastModified", "size", "type"];

pub(crate) async fn build_database(link: Scope<App>) {
    let factory = match Factory::<OurError>::get() {
        Ok(f) => f,
        Err(e) => {
            error!("Can not get factory: {e:?}");
            return;
        }
    };

    fn create_store_with_index(
        db: &Database<OurError>,
        object_store: &str,
    ) -> Result<(), Error<OurError>> {
        let store = db
            .build_object_store(object_store)
            .auto_increment()
            .create()?;
        store
            .build_compound_index(INDEX, &FILE_INDEX)
            .unique()
            .create()
            .inspect_err(|e| error!("could not build unique {object_store} index: {e:?}"))?;
        Ok(())
    }

    match factory
        .open(DB_NAME, 3, |evt| async move {
            let db = evt.database();
            if evt.old_version() == 0 {
                create_store_with_index(db, BUTTONS)?;
            }
            if evt.old_version() <= 1 {
                create_store_with_index(db, BACKGROUNDS)?;
            }
            if evt.old_version() <= 2 {
                create_store_with_index(db, STYLES)?;
            }
            Ok(())
        })
        .await
    {
        Err(e) => error!("Could not build database: {e:?}"),
        Ok(db) => link.send_message(Msg::DbBuilt(db)),
    }
}

fn create_object_url_with_blob(blob: &Blob) -> Result<ObjectUrl, JsValue> {
    Url::create_object_url_with_blob(blob).map(ObjectUrl)
}

pub(crate) fn read_assets(db: &Database<OurError>, link: Scope<App>) {
    static STORE_NAMES: [&str; 3] = [BUTTONS, BACKGROUNDS, STYLES];
    // FWIW, I'd prefer to use map, rather than mut results and for, but I
    // couldn't make it work.
    let mut results = Vec::with_capacity(2);
    let transaction = db.transaction(&STORE_NAMES).run(async move |t| {
        for store_name in &STORE_NAMES[0..2] {
            let store = t
                .object_store(store_name)
                .inspect_err(|e| error!("Can't get store {store_name}: {e:?}"))?;
            let files = store
                .get_all(None)
                .await
                .inspect_err(|e| error!("reading {store_name} failed: {e:?}"))?;
            results.push(
                files
                    .into_iter()
                    .filter_map(|file| match file.dyn_ref::<Blob>() {
                        None => {
                            error!("Could not turn {file:?} into Blob");
                            None
                        }
                        Some(blob) => create_object_url_with_blob(blob)
                            .inspect_err(|e| {
                                error!("Could not turn {blob:?} into object_url: {e:?}")
                            })
                            .ok(),
                    })
                    .collect(),
            );
        }
        let [buttons, backgrounds]: [_; 2] = results.try_into().unwrap();
        let store = t
            .object_store(STYLES)
            .inspect_err(|e| error!("Can't get store {STYLES}: {e:?}"))?;
        let files = store
            .get_all(None)
            .await
            .inspect_err(|e| error!("reading {STYLES} failed: {e:?}"))?;

        let mut styles = Vec::with_capacity(files.len());
        for file in files {
            if let Some(blob) = file.dyn_ref::<Blob>()
                && let Ok(result) = JsFuture::from(blob.text()).await
                && let Some(style) = result.as_string()
            {
                styles.push(style);
            }
        }

        let assets = Assets {
            buttons,
            backgrounds,
            styles,
        };
        link.send_message(Msg::AssetsRead(assets));
        Ok(())
    });
    spawn_local(async move {
        if let Err(e) = transaction.await {
            error!("Could not read assets: {e:?}");
        }
    });
}

fn store_asset(db: &Database<OurError>, file: File) {
    let store = STYLES;
    let transaction = db.transaction(&[store]).rw().run(move |t| async move {
        let store = t
            .object_store(store)
            .inspect_err(|e| error!("Can't get store to write styles: {e:?}"))?;
        store.add(&file).await.inspect_err(|e| {
            // TODO: don't use inspect_err
            if let Error::AlreadyExists = e {
                info!("That style is already stored");
            } else {
                error!("Could not store style: {e:?}");
            }
        })
    });
    spawn_local(async move {
        if let Err(e) = transaction.await {
            error!("Could not store style: {e:?}");
        }
    });
}

impl Component for App {
    type Message = Msg;
    type Properties = Props;

    fn create(ctx: &Context<Self>) -> Self {
        spawn_local(build_database(ctx.link().clone()));
        Default::default()
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        use Msg::*;

        match msg {
            DbBuilt(db) => {
                read_assets(&db, ctx.link().clone());
                self.db = Some(db);
            }
            Msg::AssetsRead(assets) => info!("{assets:?}"),
            Msg::StoreAsset(file) => {
                if let Some(db) = &self.db {
                    store_asset(db, file);
                }
            }
        }
        false
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link().clone();
        let onchange: Callback<Event> = (move |e: Event| match e.target() {
            None => error!("{e:?} has no target"),
            Some(target) => match target.dyn_into::<HtmlInputElement>() {
                Err(target) => error!("Could not change {target:?} into HtmlInputElement"),
                Ok(input) => match input.files() {
                    None => info!("No files"),
                    Some(files) => {
                        if let Some(file) = files.get(0) {
                            link.send_message(Msg::StoreAsset(file));
                        } else {
                            info!("No file selected");
                        }
                    }
                },
            },
        })
        .into();

        html! {
            <main>
                <img class="logo" src="https://yew.rs/img/logo.svg" alt="Yew logo" />
                <h1><input type="file" {onchange} /></h1>
                <span class="subtitle">{ "from Yew with " }<i class="heart" /></span>
            </main>
        }
    }
}
