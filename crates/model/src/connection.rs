use anyhow::Context;

pub enum Connection {
    Sqlite(sqlx::SqlitePool),
    LibSql(libsql::Database),
}

impl From<sqlx::SqlitePool> for Connection {
    fn from(pool: sqlx::SqlitePool) -> Self {
        Self::Sqlite(pool)
    }
}

impl From<libsql::Database> for Connection {
    fn from(db: libsql::Database) -> Self {
        Self::LibSql(db)
    }
}

impl Connection {
    pub async fn execute<P>(&self, query: &str, params: P) -> anyhow::Result<u64>
    where
        P: for<'q> Params<'q> + libsql::params::IntoParams,
    {
        match self {
            Self::Sqlite(pool) => {
                let result = params
                    .apply_to(sqlx::query(query))
                    .context("failed to bind parameeters")?
                    .execute(pool)
                    .await
                    .map_err(|err| anyhow::anyhow!(err))?;
                Ok(result.rows_affected())
            }

            Self::LibSql(db) => {
                let conn = db.connect().context("failed to connect to database")?;
                conn.execute(query, params)
                    .await
                    .map_err(|err| anyhow::anyhow!(err))
            }
        }
    }

    pub async fn query_scalar<T, P>(&self, query: &str, params: P) -> anyhow::Result<T>
    where
        P: for<'q> Params<'q> + libsql::params::IntoParams,
        T: Send + Unpin + serde::de::DeserializeOwned,
        (T,): for<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow>,
    {
        match self {
            Self::Sqlite(pool) => params
                .apply_to(sqlx::query_scalar(query))
                .context("failed to bind parameters")?
                .fetch_one(pool)
                .await
                .map_err(|err| anyhow::anyhow!(err)),
            Self::LibSql(db) => {
                let conn = db.connect().context("failed to connect to database")?;
                let mut rows = conn
                    .query(query, params)
                    .await
                    .context("failed to query database")?;

                let Some(row) = rows.next().await.context("failed to get first row")? else {
                    return Err(anyhow::anyhow!("query did not return any rows"));
                };

                Ok(libsql::de::from_row::<(T,)>(&row)
                    .context("failed to deserialize row")?
                    .0)
            }
        }
    }

    pub async fn query_scalars<T, P>(&self, query: &str, params: P) -> anyhow::Result<Vec<T>>
    where
        P: for<'q> Params<'q> + libsql::params::IntoParams,
        T: Send + Unpin + serde::de::DeserializeOwned,
        (T,): for<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow>,
    {
        match self {
            Self::Sqlite(pool) => params
                .apply_to(sqlx::query_scalar(query))
                .context("failed to bind parameters")?
                .fetch_all(pool)
                .await
                .map_err(|err| anyhow::anyhow!(err)),
            Self::LibSql(db) => {
                let conn = db.connect().context("failed to connect to database")?;
                let mut rows = conn
                    .query(query, params)
                    .await
                    .context("failed to query database")?;

                let mut results = Vec::new();
                while let Some(row) = rows.next().await.context("failed to get first row")? {
                    results.push(
                        libsql::de::from_row::<(T,)>(&row)
                            .context("failed to deserialize row")?
                            .0,
                    );
                }

                Ok(results)
            }
        }
    }
}

pub trait Params<'q> {
    fn apply_to<Q: ApplyParams<'q>>(self, query: Q) -> anyhow::Result<Q>;
}

macro_rules! impl_params {
    ($name:ident, $($n:ident),*) => {
        impl<'q, $($n: 'q + sqlx::Type<sqlx::Sqlite> + sqlx::Encode<'q, sqlx::Sqlite>,)*> Params<'q> for ($($n,)*) {
            fn apply_to<Q>(self, query: Q) -> anyhow::Result<Q>
            where
                Q: ApplyParams<'q>,
            {
                query.$name(self)
            }
        }
    }
}

impl<'q> Params<'q> for () {
    fn apply_to<Q>(self, query: Q) -> anyhow::Result<Q> {
        Ok(query)
    }
}

impl<'q, A> Params<'q> for (A,)
where
    A: 'q + sqlx::Type<sqlx::Sqlite> + sqlx::Encode<'q, sqlx::Sqlite>,
{
    fn apply_to<Q>(self, query: Q) -> anyhow::Result<Q>
    where
        Q: ApplyParams<'q>,
    {
        query.apply_params_1(self)
    }
}

impl_params!(apply_params_2, A, B);
impl_params!(apply_params_3, A, B, C);
impl_params!(apply_params_4, A, B, C, D);
impl_params!(apply_params_5, A, B, C, D, E);
impl_params!(apply_params_6, A, B, C, D, E, F);
impl_params!(apply_params_7, A, B, C, D, E, F, G);
impl_params!(apply_params_8, A, B, C, D, E, F, G, H);
impl_params!(apply_params_9, A, B, C, D, E, F, G, H, I);
impl_params!(apply_params_10, A, B, C, D, E, F, G, H, I, J);
impl_params!(apply_params_11, A, B, C, D, E, F, G, H, I, J, K);
impl_params!(apply_params_12, A, B, C, D, E, F, G, H, I, J, K, L);

macro_rules! decl_apply_params {
    ($name:ident, $($n:ident),*) => {
        fn $name<$($n: 'q + sqlx::Type<sqlx::Sqlite> + sqlx::Encode<'q, sqlx::Sqlite>,)*>(self, params: ($($n,)*)) -> anyhow::Result<Self>;
    }
}

macro_rules! impl_apply_params {
    ($name:ident, $($i:ident : $n:ident),*) => {
        fn $name<$($n: 'q + sqlx::Type<sqlx::Sqlite> + sqlx::Encode<'q, sqlx::Sqlite>,)*>(self, ($($i,)*): ($($n,)*)) -> anyhow::Result<Self> {
            let mut query = self;

            $(
                query = query.bind($i);
            )*

            Ok(query)
        }
    };
}

pub trait ApplyParams<'q>: Sized {
    decl_apply_params!(apply_params_1, A);
    decl_apply_params!(apply_params_2, A, B);
    decl_apply_params!(apply_params_3, A, B, C);
    decl_apply_params!(apply_params_4, A, B, C, D);
    decl_apply_params!(apply_params_5, A, B, C, D, E);
    decl_apply_params!(apply_params_6, A, B, C, D, E, F);
    decl_apply_params!(apply_params_7, A, B, C, D, E, F, G);
    decl_apply_params!(apply_params_8, A, B, C, D, E, F, G, H);
    decl_apply_params!(apply_params_9, A, B, C, D, E, F, G, H, I);
    decl_apply_params!(apply_params_10, A, B, C, D, E, F, G, H, I, J);
    decl_apply_params!(apply_params_11, A, B, C, D, E, F, G, H, I, J, K);
    decl_apply_params!(apply_params_12, A, B, C, D, E, F, G, H, I, J, K, L);
}

impl<'q, T> ApplyParams<'q>
    for sqlx::query::QueryScalar<'q, sqlx::Sqlite, T, sqlx::sqlite::SqliteArguments<'q>>
{
    impl_apply_params!(apply_params_1,  a: A);
    impl_apply_params!(apply_params_2,  a: A, b: B);
    impl_apply_params!(apply_params_3,  a: A, b: B, c: C);
    impl_apply_params!(apply_params_4,  a: A, b: B, c: C, d: D);
    impl_apply_params!(apply_params_5,  a: A, b: B, c: C, d: D, e: E);
    impl_apply_params!(apply_params_6,  a: A, b: B, c: C, d: D, e: E, f: F);
    impl_apply_params!(apply_params_7,  a: A, b: B, c: C, d: D, e: E, f: F, g: G);
    impl_apply_params!(apply_params_8,  a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: H);
    impl_apply_params!(apply_params_9,  a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: H, i: I);
    impl_apply_params!(apply_params_10, a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: H, i: I, j: J);
    impl_apply_params!(apply_params_11, a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: H, i: I, j: J, k: K);
    impl_apply_params!(apply_params_12, a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: H, i: I, j: J, k: K, l: L);
}

impl<'q> ApplyParams<'q>
    for sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>>
{
    impl_apply_params!(apply_params_1,  a: A);
    impl_apply_params!(apply_params_2,  a: A, b: B);
    impl_apply_params!(apply_params_3,  a: A, b: B, c: C);
    impl_apply_params!(apply_params_4,  a: A, b: B, c: C, d: D);
    impl_apply_params!(apply_params_5,  a: A, b: B, c: C, d: D, e: E);
    impl_apply_params!(apply_params_6,  a: A, b: B, c: C, d: D, e: E, f: F);
    impl_apply_params!(apply_params_7,  a: A, b: B, c: C, d: D, e: E, f: F, g: G);
    impl_apply_params!(apply_params_8,  a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: H);
    impl_apply_params!(apply_params_9,  a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: H, i: I);
    impl_apply_params!(apply_params_10, a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: H, i: I, j: J);
    impl_apply_params!(apply_params_11, a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: H, i: I, j: J, k: K);
    impl_apply_params!(apply_params_12, a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: H, i: I, j: J, k: K, l: L);
}
