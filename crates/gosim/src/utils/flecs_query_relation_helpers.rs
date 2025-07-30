use flecs_ecs::prelude::{
    system::SystemBuilder, Access, FromAccessArg, QueryBuilder, QueryBuilderImpl, QueryTuple,
    SingleAccessArg, TermBuilderImpl,
};

pub trait FlecsQueryRelationHelpers<'a>: QueryBuilderImpl<'a> + TermBuilderImpl<'a> {
    /// Adds a relationship constraint to a query equivalent to:
    /// ```rust
    /// q.with(rel).set_src(src).set_second(second)
    /// ```
    /// If `src` is the special token `This` then the "$this" entity is used which de facto means
    /// the `set_src` call is not executed. Similarly for the third argument.
    fn related<T1, T2, T3>(&mut self, src: T2, rel: T1, second: T3) -> &mut Self
    where
        Access: FromAccessArg<T1>,
        T2: QueryArgApply,
        T3: QueryArgApply,
    {
        self.with(rel);
        src.set_as_src(self);
        second.set_as_second(self);
        self
    }

    // fn unrelated<T1, T2, T3>(&mut self, src: T2, rel: T1, second: T3) -> &mut Self
    // where
    //     Access: FromAccessArg<T1>,
    //     T2: QueryArgApply,
    //     T3: QueryArgApply,
    // {
    //     self.not();
    //     self.with(rel);
    //     src.set_as_src(self);
    //     second.set_as_second(self);
    //     self
    // }

    /// Adds a relationship constraint to a query equivalent to:
    /// ```rust
    /// q.with(rel).set_src(src)
    /// ```
    /// If `src` is the special token `This` then the "$this" entity is used which de facto means
    /// the `set_src` call is not executed.
    fn tagged<T1, T2>(&mut self, src: T2, rel: T1) -> &mut Self
    where
        T1: QueryRelApply,
        T2: QueryArgApply,
    {
        rel.select(self);
        src.set_as_src(self);
        self
    }

    fn singleton_at(&mut self, index: u32) -> &mut Self {
        self.term_at(index).singleton()
    }
}

impl<'a, T: QueryTuple> FlecsQueryRelationHelpers<'a> for QueryBuilder<'a, T> {}

impl<'a, T: QueryTuple> FlecsQueryRelationHelpers<'a> for SystemBuilder<'a, T> {}

pub trait QueryArgApply {
    fn set_as_src<'a>(self, builder: &mut impl QueryBuilderImpl<'a>);
    fn set_as_second<'a>(self, builder: &mut impl QueryBuilderImpl<'a>);
}

pub struct This;

impl<T> QueryArgApply for T
where
    Access: FromAccessArg<T>,
    T: SingleAccessArg,
{
    fn set_as_src<'a>(self, builder: &mut impl QueryBuilderImpl<'a>) {
        builder.set_src(self);
    }
    fn set_as_second<'a>(self, builder: &mut impl QueryBuilderImpl<'a>) {
        builder.set_second(self);
    }
}

impl QueryArgApply for This {
    fn set_as_src<'a>(self, builder: &mut impl QueryBuilderImpl<'a>) {
        builder.set_src("$this");
    }
    fn set_as_second<'a>(self, builder: &mut impl QueryBuilderImpl<'a>) {
        builder.set_second("$this");
    }
}

pub trait QueryRelApply {
    fn select<'a>(self, builder: &mut impl QueryBuilderImpl<'a>);
}

impl<T> QueryRelApply for T
where
    Access: FromAccessArg<T>,
{
    fn select<'a>(self, builder: &mut impl QueryBuilderImpl<'a>) {
        builder.with(self);
    }
}

pub struct Arg(pub u32);

impl QueryRelApply for Arg {
    fn select<'a>(self, builder: &mut impl QueryBuilderImpl<'a>) {
        builder.term_at(self.0);
    }
}

// pub trait FlecsSystemBuilderEachF<'a, P, T>: QueryBuilderImpl<'a> + TermBuilderImpl<'a>
// where
//     Self: Sized,
//     P: flecs_ecs::core::ComponentId,
//     T: QueryTuple,
// {
//     fn each_f<F>(&mut self, f: F)
//     where
//         T: FlecsSystemBuilderEachFReq<Self, P, F>,
//     {
//         <T as FlecsSystemBuilderEachFReq<Self, P, F>>::apply(self, f)
//     }
// }

// impl<'a, P: ComponentId, T: QueryTuple> FlecsSystemBuilderEachF<'a, P, T> for QueryBuilder<'a, T>
// {}

// impl<'a, P: ComponentId, T: QueryTuple> FlecsSystemBuilderEachF<'a, P, T> for SystemBuilder<'a,
// T> {}

// pub trait FlecsSystemBuilderEachFReq<B, P, F> {
//     fn apply(b: &mut B, f: F);
// }

// impl<'a, B, P, F, A1, A2> FlecsSystemBuilderEachFReq<B, P, F> for (A1, A2)
// where
//     B: SystemAPI<'a, P, (A1, A2)>,
//     P: ComponentId,
//     F: FnMut(A1, A2) + 'static,
//     (A1, A2): for<'q> QueryTuple<TupleType<'q> = (A1, A2)>,
// {
//     fn apply(b: &mut B, mut f: F) {
//         b.each(move |(a1, a2)| f(a1, a2));
//     }
// }

// impl<'a, B, P, F, A1, A2, A3> FlecsSystemBuilderEachFReq<B, P, F> for (A1, A2, A3)
// where
//     B: SystemAPI<'a, P, (A1, A2, A3)>,
//     P: ComponentId,
//     F: FnMut(A1, A2, A3) + 'static,
//     (A1, A2, A3): for<'q> QueryTuple<TupleType<'q> = (A1, A2, A3)>,
// {
//     fn apply(b: &mut B, mut f: F) {
//         b.each(move |(a1, a2, a3)| f(a1, a2, a3));
//     }
// }

// impl<'a, 't, B, P, F, A1, A2, A3, A4> FlecsSystemBuilderEachFReq<B, P, F> for (A1, A2, A3, A4)
// where
//     B: SystemAPI<'a, P, (A1, A2, A3, A4)>,
//     P: ComponentId,
//     F: FnMut(A1, A2, A3, A4) + 'static,
//     (A1, A2, A3, A4): QueryTuple<TupleType<'t> = (A1, A2, A3, A4)>,
// {
//     fn apply(b: &mut B, mut f: F) {
//         b.each(move |(a1, a2, a3, a4)| f(a1, a2, a3, a4));
//     }
// }

// pub trait FlecsSystemBuilderEachF2<'a, A1, A2>: QueryBuilderImpl<'a> + TermBuilderImpl<'a> {
//     fn each_f<F>(&mut self, mut f: F)
//     where
//         F: FnMut(A1, A2),
//     {
//         self.each_f(|a1, a2| f(a1, a2))
//     }
// }

// impl<'a, A1, A2> FlecsSystemBuilderEachF2<'a, A1, A2> for SystemBuilder<'a, (A1, A2)> where
//     (A1, A2): QueryTuple
// {
// }

// pub trait FlecsSystemBuilderEachF4<'a, A1, A2, A3, A4>:
//     QueryBuilderImpl<'a> + TermBuilderImpl<'a>
// {
//     fn each_f<F>(&mut self, mut f: F)
//     where
//         F: FnMut(A1, A2, A3, A4),
//     {
//         self.each_f(|a1, a2, a3, a4| f(a1, a2, a3, a4))
//     }
// }

// impl<'a, A1, A2, A3, A4> FlecsSystemBuilderEachF4<'a, A1, A2, A3, A4>
//     for SystemBuilder<'a, (A1, A2, A3, A4)>
// where
//     (A1, A2, A3, A4): QueryTuple,
// {
// }

// macro_rules! impl_flecs_system_builder_each_fn {
//     // Entry point
//     () => {
//         impl_flecs_system_builder_each_fn! { 1 => A1 => a1 }
//         impl_flecs_system_builder_each_fn! { 2 => A1, A2 => a1, a2 }
//         impl_flecs_system_builder_each_fn! { 3 => A1, A2, A3 => a1, a2, a3 }
//         impl_flecs_system_builder_each_fn! { 4 => A1, A2, A3, A4 => a1, a2, a3, a4 }
//         impl_flecs_system_builder_each_fn! { 5 => A1, A2, A3, A4, A5 => a1, a2, a3, a4, a5 }
//         impl_flecs_system_builder_each_fn! { 6 => A1, A2, A3, A4, A5, A6 => a1, a2, a3, a4, a5,
// a6 }         impl_flecs_system_builder_each_fn! { 7 => A1, A2, A3, A4, A5, A6, A7 => a1, a2, a3,
// a4, a5, a6, a7 }         impl_flecs_system_builder_each_fn! { 8 => A1, A2, A3, A4, A5, A6, A7, A8
// => a1, a2, a3, a4, a5, a6, a7, a8 }     };

//     // Per-arity expansion
//     ($n:literal => $($A:ident),+ => $($a:ident),+) => {
//         paste::paste! {
//             pub trait [<FlecsSystemBuilderEachF $n>]<'a, $($A),*>:
//                 QueryBuilderImpl<'a> + TermBuilderImpl<'a>
//             {
//                 fn each_f<F>(&mut self, mut f: F)
//                 where
//                     F: FnMut($($A),*),
//                 {
//                     self.each_f(|$($a),*| f($($a),*))
//                 }
//             }

//             impl<'a, $($A),*> [<FlecsSystemBuilderEachF $n>]<'a, $($A),*>
//                 for SystemBuilder<'a, ($($A),*)>
//             where
//                 ($($A),*): QueryTuple,
//             {
//             }
//         }
//     };
// }

// // You must call the macro to generate the code
// impl_flecs_system_builder_each_fn!();
