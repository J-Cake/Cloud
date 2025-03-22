use actix_web::HttpResponse;
use actix_web::Responder;
use actix_web::Result;
use actix_web::get;
use actix_web::web::Data;
use actix_web::web::Path;
use sqlx::PgPool;
use std::collections::HashMap;
use std::fs;
use std::fs::DirEntry;
use std::ops::Deref;
use std::path::PathBuf;

#[derive(Clone)]
pub struct SqlMap {
    map: HashMap<PathBuf, SqlQuery>,
}

impl Deref for SqlMap {
    type Target = HashMap<PathBuf, SqlQuery>;

    fn deref(&self) -> &Self::Target {
        &self.map
    }
}

impl SqlMap {
    pub fn new(search: impl AsRef<std::path::Path>) -> Result<Self> {
        let mut map = HashMap::new();

        fn walk(dir: PathBuf, map: &mut HashMap<PathBuf, SqlQuery>, search: &std::path::Path) -> Result<()> {
            for i in fs::read_dir(dir)? {
                match i? {
                    i if i.metadata()?.is_file() && i.path().extension().is_some_and(|i| i.eq("sql")) => {
                        map.insert(
                            i.path()
                                .strip_prefix(search)
	                            .expect("Failed to make path relative to base")
                                .to_owned(),
                            fs::read_to_string(i.path())?,
                        );
                    }
                    i if i.metadata()?.is_dir() => walk(i.path(), map, search)?,
                    _ => (),
                }
            }

            Ok(())
        }

        walk(search.as_ref().to_owned(), &mut map, search.as_ref())?;

        Ok(Self { map })
    }
}

pub type SqlQuery = String;

#[get("/method/{method}")]
pub async fn method(pool: Data<PgPool>, map: Data<SqlMap>, path: Path<(PathBuf,)>) -> Result<impl Responder> {
    let (method,) = path.into_inner();

    if let Some(sql) = map.get(&method) {
        Ok(HttpResponse::Ok())
    } else {
        Ok(HttpResponse::NotFound())
    }
}
