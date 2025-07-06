use {
    indexed_db::{Database, Factory},
    log::{error, info},
    wasm_bindgen::JsCast,
    wasm_bindgen_futures::JsFuture,
    web_sys::{Blob, Event, File, HtmlInputElement},
    yew::{html::Scope, platform::spawn_local, prelude::*},
};

type DbError = ();
type Props = ();

#[derive(Default)]
pub(crate) struct App {
    db: Option<Database<DbError>>,
}

pub(crate) enum Msg {
    DbBuilt(Database<DbError>),
    StoreStyle(File),
}

const DB_NAME: &str = "indexed-db-panic";
const STYLES: &str = "styles";

pub(crate) async fn build_database(link: Scope<App>) {
    let factory = match Factory::<DbError>::get() {
        Ok(f) => f,
        Err(e) => {
            error!("Can not get factory: {e:?}");
            return;
        }
    };

    match factory
        .open(DB_NAME, 1, |evt| async move {
            let db = evt.database();
            if evt.old_version() == 0 {
                db
                    .build_object_store(STYLES)
                    .auto_increment()
                    .create()?;
            }
            Ok(())
        })
        .await
    {
        Err(e) => error!("Could not build database: {e:?}"),
        Ok(db) => link.send_message(Msg::DbBuilt(db)),
    }
}

pub(crate) fn read_styles(db: &Database<DbError>) {
    let transaction = db.transaction(&[STYLES]).run(async move |t| {
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
        info!("styles: {styles:?}");
        Ok(())
    });

    spawn_local(async move {
        if let Err(e) = transaction.await {
            error!("Could not read styles: {e:?}");
        }
    });
}

fn store_style(db: &Database<DbError>, file: File) {
    let store = STYLES;
    let transaction = db.transaction(&[STYLES]).rw().run(move |t| async move {
        let store = t
            .object_store(store)
            .inspect_err(|e| error!("Can't get store to write styles: {e:?}"))?;
        store.add(&file).await.inspect_err(|e| {
            error!("Could not store style: {e:?}");
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

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        use Msg::*;

        match msg {
            DbBuilt(db) => {
                read_styles(&db);
                self.db = Some(db);
            }
            Msg::StoreStyle(file) => {
                if let Some(db) = &self.db {
                    store_style(db, file);
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
                            link.send_message(Msg::StoreStyle(file));
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
