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

impl<'q, T, const N: usize> Params<'q> for [T; N]
where
    T: 'q + sqlx::Type<sqlx::Sqlite> + sqlx::Encode<'q, sqlx::Sqlite>,
{
    fn apply_to<Q>(self, query: Q) -> anyhow::Result<Q>
    where
        Q: ApplyParams<'q>,
    {
        query.apply_params_v(self)
    }
}

impl<'q, T> Params<'q> for Vec<T>
where
    T: 'q + sqlx::Type<sqlx::Sqlite> + sqlx::Encode<'q, sqlx::Sqlite>,
{
    fn apply_to<Q>(self, query: Q) -> anyhow::Result<Q>
    where
        Q: ApplyParams<'q>,
    {
        query.apply_params_vec(self)
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
impl_params!(apply_params_13, A, B, C, D, E, F, G, H, I, J, K, L, M);
impl_params!(apply_params_14, A, B, C, D, E, F, G, H, I, J, K, L, M, N);
impl_params!(apply_params_15, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);

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
    fn apply_params_v<
        A: 'q + sqlx::Type<sqlx::Sqlite> + sqlx::Encode<'q, sqlx::Sqlite>,
        const N: usize,
    >(
        self,
        params: [A; N],
    ) -> anyhow::Result<Self>;

    fn apply_params_vec<A: 'q + sqlx::Type<sqlx::Sqlite> + sqlx::Encode<'q, sqlx::Sqlite>>(
        self,
        params: Vec<A>,
    ) -> anyhow::Result<Self>;

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
    decl_apply_params!(apply_params_13, A, B, C, D, E, F, G, H, I, J, K, L, M);
    decl_apply_params!(apply_params_14, A, B, C, D, E, F, G, H, I, J, K, L, M, N);
    decl_apply_params!(apply_params_15, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
}

impl<'q, T> ApplyParams<'q>
    for sqlx::query::QueryScalar<'q, sqlx::Sqlite, T, sqlx::sqlite::SqliteArguments<'q>>
{
    fn apply_params_v<
        A: 'q + sqlx::Type<sqlx::Sqlite> + sqlx::Encode<'q, sqlx::Sqlite>,
        const N: usize,
    >(
        self,
        params: [A; N],
    ) -> anyhow::Result<Self> {
        let mut query = self;

        for param in params.into_iter() {
            query = query.bind(param);
        }

        Ok(query)
    }

    fn apply_params_vec<A: 'q + sqlx::Type<sqlx::Sqlite> + sqlx::Encode<'q, sqlx::Sqlite>>(
        self,
        params: Vec<A>,
    ) -> anyhow::Result<Self> {
        let mut query = self;

        for param in params.into_iter() {
            query = query.bind(param);
        }

        Ok(query)
    }

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
    impl_apply_params!(apply_params_13, a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: H, i: I, j: J, k: K, l: L, m: M);
    impl_apply_params!(apply_params_14, a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: H, i: I, j: J, k: K, l: L, m: M, n: N);
    impl_apply_params!(apply_params_15, a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: H, i: I, j: J, k: K, l: L, m: M, n: N, o: O);
}

impl<'q> ApplyParams<'q>
    for sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>>
{
    fn apply_params_v<
        A: 'q + sqlx::Type<sqlx::Sqlite> + sqlx::Encode<'q, sqlx::Sqlite>,
        const N: usize,
    >(
        self,
        params: [A; N],
    ) -> anyhow::Result<Self> {
        let mut query = self;

        for param in params.into_iter() {
            query = query.bind(param);
        }

        Ok(query)
    }

    fn apply_params_vec<A: 'q + sqlx::Type<sqlx::Sqlite> + sqlx::Encode<'q, sqlx::Sqlite>>(
        self,
        params: Vec<A>,
    ) -> anyhow::Result<Self> {
        let mut query = self;

        for param in params.into_iter() {
            query = query.bind(param);
        }

        Ok(query)
    }

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
    impl_apply_params!(apply_params_13, a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: H, i: I, j: J, k: K, l: L, m: M);
    impl_apply_params!(apply_params_14, a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: H, i: I, j: J, k: K, l: L, m: M, n: N);
    impl_apply_params!(apply_params_15, a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: H, i: I, j: J, k: K, l: L, m: M, n: N, o: O);
}

impl<'q, O> ApplyParams<'q>
    for sqlx::query::QueryAs<'q, sqlx::Sqlite, O, sqlx::sqlite::SqliteArguments<'q>>
{
    fn apply_params_v<
        A: 'q + sqlx::Type<sqlx::Sqlite> + sqlx::Encode<'q, sqlx::Sqlite>,
        const N: usize,
    >(
        self,
        params: [A; N],
    ) -> anyhow::Result<Self> {
        let mut query = self;

        for param in params.into_iter() {
            query = query.bind(param);
        }

        Ok(query)
    }

    fn apply_params_vec<A: 'q + sqlx::Type<sqlx::Sqlite> + sqlx::Encode<'q, sqlx::Sqlite>>(
        self,
        params: Vec<A>,
    ) -> anyhow::Result<Self> {
        let mut query = self;

        for param in params.into_iter() {
            query = query.bind(param);
        }

        Ok(query)
    }

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
    impl_apply_params!(apply_params_13, a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: H, i: I, j: J, k: K, l: L, m: M);
    impl_apply_params!(apply_params_14, a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: H, i: I, j: J, k: K, l: L, m: M, n: N);
    impl_apply_params!(apply_params_15, a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: H, i: I, j: J, k: K, l: L, m: M, n: N, p: P);
}
