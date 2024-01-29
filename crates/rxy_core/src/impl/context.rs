use crate::{
    ElementView, IntoView, MemberOwner, Renderer, RendererNodeId, SoloView, View, ViewCtx, ViewKey,
    ViewMember,
};
use rxy_macro::IntoView;
use core::marker::PhantomData;

pub struct ProvideContext<R, T, V> {
    provide_context: T,
    view: V,
    _marker: PhantomData<R>,
}

pub fn provide_context<R, T, IV>(provide_context: T, view: IV) -> ProvideContext<R, T, IV::View>
    where
        R: Renderer,
        T: Send + Sync + 'static,
        IV: IntoView<R>,
        IV::View: SoloView<R>,
{
    ProvideContext {
        provide_context,
        view: view.into_view(),
        _marker: Default::default(),
    }
}

impl<R> ViewCtx<'_, R>
    where
        R: Renderer,
{
    pub fn context<T: Send + Sync + Clone + 'static>(&self) -> T {
        self.get_context()
            .unwrap_or_else(|| panic!("Tried to access a context that has not been provided."))
    }

    pub fn context_ref<T: Send + Sync + 'static>(&self) -> &T {
        self.get_context_ref()
            .unwrap_or_else(|| panic!("Tried to access a context that has not been provided."))
    }

    pub fn get_context<T: Send + Sync + Clone + 'static>(&self) -> Option<T> {
        self.get_context_ref().cloned()
    }

    pub fn get_context_ref<T: Send + Sync + 'static>(&self) -> Option<&T> {
        let mut current_parent = self.parent.clone();
        loop {
            if let Some(context) = R::get_state_ref::<Context<T>>(self.world, &current_parent) {
                return Some(&context.0);
            }
            if let Some(parent) = R::get_parent(self.world, &current_parent) {
                current_parent = parent;
            } else {
                return None;
            }
        }
    }
    pub fn context_scoped<T: Send + Sync + 'static>(&mut self, f: impl FnOnce(&mut T)) -> bool {
        let mut current_parent = self.parent.clone();
        loop {
            if let Some(mut context) = R::take_state::<Context<T>>(self.world, &current_parent) {
                f(&mut context.0);
                R::set_state(self.world, &current_parent, context);
                return true;
            }
            if let Some(parent) = R::get_parent(self.world, &current_parent) {
                current_parent = parent;
            } else {
                return false;
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Context<T>(pub T);

impl<R, T, V> View<R> for ProvideContext<R, T, V>
    where
        R: Renderer,
        T: Send + Sync + 'static,
        V: ElementView<R>,
{
    type Key = V::Key;

    fn build(
        self,
        ctx: ViewCtx<R>,
        reserve_key: Option<Self::Key>,
        will_rebuild: bool,
    ) -> Self::Key {
        let reserve_key =
            reserve_key.unwrap_or_else(|| V::Key::reserve_key(ctx.world, will_rebuild));
        let node_id = V::element_node_id(&reserve_key);
        R::ensure_spawn(ctx.world, node_id.clone());
        R::set_state::<Context<T>>(ctx.world, node_id, Context(self.provide_context));
        
        self.view.build(
            ViewCtx {
                world: &mut *ctx.world,
                parent: ctx.parent,
            },
            Some(reserve_key),
            will_rebuild,
        )
    }

    fn rebuild(self, ctx: ViewCtx<R>, key: Self::Key) {
        let node_id = V::element_node_id(&key);
        R::set_state::<Context<T>>(ctx.world, node_id, Context(self.provide_context));
        self.view.rebuild(ctx, key);
    }
}

impl<R, T, V> IntoView<R> for ProvideContext<R, T, V>
    where
        R: Renderer,
        T: Send + Sync + 'static,
        V: ElementView<R>,
{
    type View = ProvideContext<R, T, V>;
    fn into_view(self) -> Self::View {
        self
    }
}

impl<R, T, V> SoloView<R> for ProvideContext<R, T, V>
    where
        R: Renderer,
        T: Send + Sync + 'static,
        V: ElementView<R>,
{
    fn node_id(key: &Self::Key) -> &RendererNodeId<R> {
        V::element_node_id(key)
    }
}

impl<R, T, V> ElementView<R> for ProvideContext<R, T, V>
    where
        R: Renderer,
        T: Send + Sync + 'static,
        V: ElementView<R>,
{
    fn element_node_id(key: &Self::Key) -> &RendererNodeId<R> {
        V::element_node_id(key)
    }
}

impl<R, T, V> MemberOwner<R> for ProvideContext<R, T, V>
    where
        R: Renderer,
        T: Send + Sync + 'static,
        V: MemberOwner<R>,
{
    type E = V::E;
    type VM = V::VM;
    type AddMember<VM: ViewMember<R>> = ProvideContext<R, T, V::AddMember<VM>>;
    type SetMembers<VM: ViewMember<R> + MemberOwner<R>> = ProvideContext<R, T, V::SetMembers<VM>>;

    fn member<VM>(self, member: VM) -> Self::AddMember<VM>
        where
            (Self::VM, VM): ViewMember<R>,
            VM: ViewMember<R>,
    {
        ProvideContext {
            provide_context: self.provide_context,
            view: self.view.member(member),
            _marker: Default::default(),
        }
    }

    fn members<VM: ViewMember<R>>(self, members: VM) -> Self::SetMembers<(VM, )>
        where
            VM: ViewMember<R>,
    {
        ProvideContext {
            provide_context: self.provide_context,
            view: self.view.members(members),
            _marker: Default::default(),
        }
    }
}
