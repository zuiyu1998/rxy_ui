use core::fmt::Debug;
use core::marker::PhantomData;

use crate::element::{view_children, ElementType, ElementViewChildren};
use crate::{
    ElementView, IntoView, IntoViewMember, MemberOwner, NodeTree, Renderer,
    RendererNodeId, SoloView, View, ViewCtx, ViewKey, ViewMember,
    ViewMemberCtx,
};

#[derive(Clone)]
pub struct Element<R, E, VM> {
    pub members: VM,
    pub _marker: PhantomData<(R, E)>,
}

impl<R, E, VM> Element<R, E, VM>
    where
        R: Renderer,
        E: ElementType<R>,
        VM: ViewMember<R>,
{

    pub fn new(members: VM) -> Self {
        Self {
            members,
            _marker: Default::default(),
        }
    }

    #[cfg(not(feature = "view_erasure"))]
    pub fn children<CV>(self, children: CV) -> ElementViewChildren<R, Element<R, E, VM>, CV::View>
        where
            CV: IntoView<R>,
    {
        view_children(self, children)
    }

    #[cfg(feature = "view_erasure")]
    pub fn children<CV>(self, children: CV) -> ElementViewChildren<R, Element<R, E, VM>, crate::BoxedErasureView<R>>
        where
            CV: IntoView<R>,
    {
        use crate::IntoViewErasureExt;
        view_children(self, unsafe { children.into_erasure_view() })
    }
}

impl<R, E> Default for Element<R, E, ()>
where
    R: Renderer,
{
    fn default() -> Self {
        Self {
            members: (),
            _marker: Default::default(),
        }
    }
}

impl<R, E, VM> MemberOwner<R> for Element<R, E, VM>
where
    R: Renderer,
    E: ElementType<R>,
    VM: ViewMember<R>,
{
    type E = E;
    type VM = VM;
    type AddMember<T: ViewMember<R>> = Element<R, E, (VM, T)>;
    type SetMembers<T: ViewMember<R> + MemberOwner<R>> = Element<R, E, T>;

    fn member<T>(self, member: impl IntoViewMember<R, T>) -> Self::AddMember<T>
    where
        (VM, T): ViewMember<R>,
        T: ViewMember<R>,
    {
        Element {
            members: (self.members, member.into_member()),
            _marker: self._marker,
        }
    }

    fn members<T>(self, members: impl IntoViewMember<R, T>) -> Self::SetMembers<(T,)>
    where
        T: ViewMember<R>,
    {
        Element {
            members: (members.into_member(),),
            _marker: self._marker,
        }
    }
}

impl<R, E, VM> SoloView<R> for Element<R, E, VM>
where
    E: ElementType<R>,
    R: Renderer,
    VM: ViewMember<R>,
{
    fn node_id(key: &Self::Key) -> &RendererNodeId<R> {
        &key.0
    }
}

impl<R, E, VM> ElementView<R> for Element<R, E, VM>
where
    E: ElementType<R>,
    R: Renderer,
    VM: ViewMember<R>,
{
    fn element_node_id(key: &Self::Key) -> &RendererNodeId<R> {
        &key.0
    }
}

// #[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub struct ElementViewKey<R, VM>(
    pub RendererNodeId<R>,
    /*#[cfg_attr(feature = "bevy_reflect", reflect(ignore))] */ PhantomData<VM>,
)
where
    R: Renderer,
    VM: ViewMember<R>;

impl<R, VM> Debug for ElementViewKey<R, VM>
where
    R: Renderer,
    VM: ViewMember<R>,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("ElementViewKey").field(&self.0).finish()
    }
}

#[cfg(feature = "send_sync")]
unsafe impl<R, VM> Send for ElementViewKey<R, VM>
where
    R: Renderer,
    VM: ViewMember<R>,
{
}
#[cfg(feature = "send_sync")]
unsafe impl<R, VM> Sync for ElementViewKey<R, VM>
where
    R: Renderer,
    VM: ViewMember<R>,
{
}

impl<R, VM> Clone for ElementViewKey<R, VM>
where
    R: Renderer,
    VM: ViewMember<R>,
{
    fn clone(&self) -> Self {
        Self(self.0.clone(), Default::default())
    }
}

impl<R, VM> ViewKey<R> for ElementViewKey<R, VM>
where
    R: Renderer,
    VM: ViewMember<R>,
{
    #[inline]
    fn remove(self, world: &mut crate::RendererWorld<R>) {
        VM::unbuild(
            ViewMemberCtx {
                index: 0,
                world,
                node_id: self.0.clone(),
            },
            true,
        );
        self.0.remove(world);
    }

    #[inline]
    fn insert_before(
        &self,
        world: &mut crate::RendererWorld<R>,
        parent: Option<&RendererNodeId<R>>,
        before_node_id: Option<&RendererNodeId<R>>,
    ) {
        self.0.insert_before(world, parent, before_node_id);
    }

    fn set_visibility(&self, world: &mut crate::RendererWorld<R>, hidden: bool) {
        self.0.set_visibility(world, hidden);
    }

    fn state_node_id(&self) -> Option<RendererNodeId<R>> {
        Some(self.0.clone())
    }

    fn reserve_key(world: &mut crate::RendererWorld<R>, will_rebuild: bool) -> Self {
        Self(
            <RendererNodeId<R> as ViewKey<R>>::reserve_key(world, will_rebuild),
            Default::default(),
        )
    }

    fn first_node_id(&self, world: &crate::RendererWorld<R>) -> Option<RendererNodeId<R>> {
        self.0.first_node_id(world)
    }
}

impl<R, E, VM> View<R> for Element<R, E, VM>
where
    E: ElementType<R>,
    R: Renderer,
    VM: ViewMember<R>,
{
    type Key = ElementViewKey<R, VM>;

    fn build(
        self,
        ctx: ViewCtx<R>,
        reserve_key: Option<Self::Key>,
        will_rebuild: bool,
    ) -> Self::Key {
        let spawned_node_id = {
            let parent = ctx.parent.clone();
            ctx.world
                .spawn_node::<E>(Some(&parent), reserve_key.map(|n| n.0))
        };
        self.members.build(
            ViewMemberCtx {
                index: 0,
                world: &mut *ctx.world,
                node_id: spawned_node_id.clone(),
            },
            will_rebuild,
        );
        ElementViewKey(spawned_node_id, Default::default())
    }

    fn rebuild(self, ctx: ViewCtx<R>, state_key: Self::Key) {
        self.members.rebuild(ViewMemberCtx {
            index: 0,
            world: ctx.world,
            node_id: state_key.0,
        });
    }
}

impl<R, E, VM> IntoView<R> for Element<R, E, VM>
where
    E: ElementType<R>,
    R: Renderer,
    VM: ViewMember<R>,
{
    type View = Element<R, E, VM>;

    fn into_view(self) -> Self::View {
        self
    }
}

#[cfg(feature = "bevy_reflect")]
const _: () = {
    #[allow(unused_mut)]
    impl<R, VM> bevy_reflect::GetTypeRegistration for ElementViewKey<R, VM>
    where
        R: Renderer,
        VM: ViewMember<R>,
        RendererNodeId<R>: bevy_reflect::FromReflect + bevy_reflect::TypePath,
        R: bevy_reflect::TypePath,
    {
        fn get_type_registration() -> bevy_reflect::TypeRegistration {
            let mut registration = bevy_reflect::TypeRegistration::of::<Self>();
            registration.insert::<bevy_reflect::ReflectFromPtr>(bevy_reflect::FromType::<Self>::from_type());
            registration.insert::<bevy_reflect::ReflectFromReflect>(
                bevy_reflect::FromType::<Self>::from_type(),
            );
            registration
        }
    }
    impl<R, VM> bevy_reflect::Typed for ElementViewKey<R, VM>
    where
        R: Renderer,
        VM: ViewMember<R>,
        RendererNodeId<R>: bevy_reflect::FromReflect + bevy_reflect::TypePath,
        R: bevy_reflect::TypePath,
    {
        fn type_info() -> &'static bevy_reflect::TypeInfo {
            static CELL: bevy_reflect::utility::GenericTypeInfoCell =
                bevy_reflect::utility::GenericTypeInfoCell::new();
            CELL.get_or_insert::<Self, _>(|| {
                let fields = [bevy_reflect::UnnamedField::new::<RendererNodeId<R>>(0)];
                let info = bevy_reflect::TupleStructInfo::new::<Self>(&fields);
                bevy_reflect::TypeInfo::TupleStruct(info)
            })
        }
    }
    impl<R, VM> bevy_reflect::TypePath for ElementViewKey<R, VM>
    where
        R: Renderer,
        VM: ViewMember<R>,
        RendererNodeId<R>: bevy_reflect::FromReflect + bevy_reflect::TypePath,
        R: bevy_reflect::TypePath,
    {
        fn type_path() -> &'static str {
            static CELL: bevy_reflect::utility::GenericTypePathCell =
                bevy_reflect::utility::GenericTypePathCell::new();
            CELL.get_or_insert::<Self, _>(|| {
                ::std::string::ToString::to_string(::std::concat!(
                ::std::concat!(
                ::std::concat!(::std::module_path!(), "::"),
                "ElementViewKey"
                ),
                "<"
                )) + &::std::string::ToString::to_string(<R as bevy_reflect::TypePath>::type_path())
                    + ", "
                    // + <VM as bevy_reflect::TypePath>::type_path()
                    + "VM"
                    + ">"
            })
        }
        fn short_type_path() -> &'static str {
            static CELL: bevy_reflect::utility::GenericTypePathCell =
                bevy_reflect::utility::GenericTypePathCell::new();
            CELL.get_or_insert::<Self, _>(|| {
                ::std::string::ToString::to_string("ElementViewKey<")
                    + &::std::string::ToString::to_string(
                    <R as bevy_reflect::TypePath>::short_type_path(),
                )
                    + ", "
                    // + <VM as bevy_reflect::TypePath>::short_type_path()
                    + "VM"
                    + ">"
            })
        }
        fn type_ident() -> Option<&'static str> {
            ::std::option::Option::Some("ElementViewKey")
        }
        fn crate_name() -> Option<&'static str> {
            ::std::option::Option::Some(::std::module_path!().split(':').next().unwrap())
        }
        fn module_path() -> Option<&'static str> {
            ::std::option::Option::Some(::std::module_path!())
        }
    }
    impl<R, VM> bevy_reflect::TupleStruct for ElementViewKey<R, VM>
    where
        R: Renderer,
        VM: ViewMember<R>,
        RendererNodeId<R>: bevy_reflect::FromReflect + bevy_reflect::TypePath,
        R: bevy_reflect::TypePath,
    {
        fn field(&self, index: usize) -> ::std::option::Option<&dyn bevy_reflect::Reflect> {
            match index {
                0usize => ::std::option::Option::Some(&self.0),
                1usize => ::std::option::Option::None,
                _ => ::std::option::Option::None,
            }
        }
        fn field_mut(
            &mut self,
            index: usize,
        ) -> ::std::option::Option<&mut dyn bevy_reflect::Reflect> {
            match index {
                0usize => ::std::option::Option::Some(&mut self.0),
                1usize => ::std::option::Option::None,
                _ => ::std::option::Option::None,
            }
        }
        fn field_len(&self) -> usize {
            2usize
        }
        fn iter_fields(&self) -> bevy_reflect::TupleStructFieldIter {
            bevy_reflect::TupleStructFieldIter::new(self)
        }
        fn clone_dynamic(&self) -> bevy_reflect::DynamicTupleStruct {
            let mut dynamic: bevy_reflect::DynamicTupleStruct = ::std::default::Default::default();
            dynamic.set_represented_type(bevy_reflect::Reflect::get_represented_type_info(self));
            dynamic.insert_boxed(bevy_reflect::Reflect::clone_value(&self.0));
            // dynamic.insert_boxed(bevy_reflect::Reflect::clone_value(&self.1));
            dynamic
        }
    }
    impl<R, VM> bevy_reflect::Reflect for ElementViewKey<R, VM>
    where
        R: Renderer,
        VM: ViewMember<R>,
        RendererNodeId<R>: bevy_reflect::FromReflect + bevy_reflect::TypePath,
        R: bevy_reflect::TypePath,
    {
        #[inline]
        fn get_represented_type_info(
            &self,
        ) -> ::std::option::Option<&'static bevy_reflect::TypeInfo> {
            ::std::option::Option::Some(<Self as bevy_reflect::Typed>::type_info())
        }
        #[inline]
        fn into_any(self: ::std::boxed::Box<Self>) -> ::std::boxed::Box<dyn ::std::any::Any> {
            self
        }
        #[inline]
        fn as_any(&self) -> &dyn ::std::any::Any {
            self
        }
        #[inline]
        fn as_any_mut(&mut self) -> &mut dyn ::std::any::Any {
            self
        }
        #[inline]
        fn into_reflect(
            self: ::std::boxed::Box<Self>,
        ) -> ::std::boxed::Box<dyn bevy_reflect::Reflect> {
            self
        }
        #[inline]
        fn as_reflect(&self) -> &dyn bevy_reflect::Reflect {
            self
        }
        #[inline]
        fn as_reflect_mut(&mut self) -> &mut dyn bevy_reflect::Reflect {
            self
        }
        #[inline]
        fn clone_value(&self) -> ::std::boxed::Box<dyn bevy_reflect::Reflect> {
            ::std::boxed::Box::new(bevy_reflect::TupleStruct::clone_dynamic(self))
        }
        #[inline]
        fn set(
            &mut self,
            value: ::std::boxed::Box<dyn bevy_reflect::Reflect>,
        ) -> ::std::result::Result<(), ::std::boxed::Box<dyn bevy_reflect::Reflect>> {
            *self = <dyn bevy_reflect::Reflect>::take(value)?;
            ::std::result::Result::Ok(())
        }
        #[inline]
        fn apply(&mut self, value: &dyn bevy_reflect::Reflect) {
            if let bevy_reflect::ReflectRef::TupleStruct(struct_value) =
                bevy_reflect::Reflect::reflect_ref(value)
            {
                for (i, value) in ::std::iter::Iterator::enumerate(
                    bevy_reflect::TupleStruct::iter_fields(struct_value),
                ) {
                    bevy_reflect::TupleStruct::field_mut(self, i).map(|v| v.apply(value));
                }
            } else {
                panic!("Attempted to apply non-TupleStruct type to TupleStruct type.");
            }
        }
        fn reflect_ref(&self) -> bevy_reflect::ReflectRef {
            bevy_reflect::ReflectRef::TupleStruct(self)
        }
        fn reflect_mut(&mut self) -> bevy_reflect::ReflectMut {
            bevy_reflect::ReflectMut::TupleStruct(self)
        }
        fn reflect_owned(self: ::std::boxed::Box<Self>) -> bevy_reflect::ReflectOwned {
            bevy_reflect::ReflectOwned::TupleStruct(self)
        }
        fn reflect_partial_eq(
            &self,
            value: &dyn bevy_reflect::Reflect,
        ) -> ::std::option::Option<bool> {
            bevy_reflect::tuple_struct_partial_eq(self, value)
        }
    }
    impl<R, VM> bevy_reflect::FromReflect for ElementViewKey<R, VM>
    where
        R: Renderer,
        VM: ViewMember<R>,
        RendererNodeId<R>: bevy_reflect::FromReflect + bevy_reflect::TypePath,
        R: bevy_reflect::TypePath,
    {
        fn from_reflect(reflect: &dyn bevy_reflect::Reflect) -> ::std::option::Option<Self> {
            if let bevy_reflect::ReflectRef::TupleStruct(__ref_struct) =
                bevy_reflect::Reflect::reflect_ref(reflect)
            {
                ::std::option::Option::Some(Self {
                    0: (|| {
                        <RendererNodeId<R> as bevy_reflect::FromReflect>::from_reflect(
                            bevy_reflect::TupleStruct::field(__ref_struct, 0)?,
                        )
                    })()?,
                    1: Default::default(),
                })
            } else {
                ::std::option::Option::None
            }
        }
    }
};
