use std::{
    any::{Any, TypeId},
    hash::{Hash, Hasher},
    ops::{Deref, DerefMut},
};

use super::{
    bindings::{CommandId, DynamicBindings, NamedBindingQuery},
    layout::{ComponentKey, Layout},
    Component, ComponentLink, MessageSender, ShouldRender,
};
use crate::{terminal::Rect, KeyEvent};

#[derive(Clone, Copy, Debug)]
pub(crate) struct ComponentId {
    type_id: TypeId,
    id: u64,

    // The `type_name` field is used only for debugging -- in particular
    // note that it's not a valid unique id for a type. See
    // https://doc.rust-lang.org/std/any/fn.type_name.html
    type_name: &'static str,
}

// `PartialEq` is impl'ed manually as `type_name` is only used for
// debugging and is ignored when testing for equality.
impl PartialEq for ComponentId {
    fn eq(&self, other: &Self) -> bool {
        self.type_id == other.type_id && self.id == other.id
    }
}

impl Eq for ComponentId {}

impl Hash for ComponentId {
    fn hash<HasherT: Hasher>(&self, hasher: &mut HasherT) {
        self.type_id.hash(hasher);
        self.id.hash(hasher);
    }
}

impl ComponentId {
    #[inline]
    pub(crate) fn new<T: 'static>(id: u64) -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            type_name: std::any::type_name::<T>(),
            id,
        }
    }

    #[inline]
    pub(crate) fn type_id(&self) -> TypeId {
        self.type_id
    }

    pub(crate) fn type_name(&self) -> &'static str {
        self.type_name
    }
}

impl std::fmt::Display for ComponentId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "{} / {:x}", self.type_name(), self.id >> 32)
    }
}

pub(crate) struct DynamicMessage(pub(crate) Box<dyn Any + Send + 'static>);
pub(crate) struct DynamicProperties(Box<dyn Any>);
// pub(crate) struct DynamicBindings(pub(crate) Box<dyn HasKeymap>);
pub(crate) struct DynamicTemplate(pub(crate) Box<dyn Template>);

impl Deref for DynamicTemplate {
    type Target = dyn Template;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl DerefMut for DynamicTemplate {
    fn deref_mut(&mut self) -> &mut <Self as Deref>::Target {
        self.0.deref_mut()
    }
}

pub(crate) trait Renderable {
    fn change(&mut self, properties: DynamicProperties) -> ShouldRender;

    fn resize(&mut self, frame: Rect) -> ShouldRender;

    fn update(&mut self, message: DynamicMessage) -> ShouldRender;

    fn view(&self) -> Layout;

    fn bindings(&self, bindings: &mut DynamicBindings);

    fn notify_binding_queries(&self, bindings: &[Option<NamedBindingQuery>], keys: &[KeyEvent]);

    fn run_command(
        &self,
        bindings: &DynamicBindings,
        command_id: CommandId,
        pressed: &[KeyEvent],
    ) -> Option<DynamicMessage>;

    fn tick(&self) -> Option<DynamicMessage>;
}

impl<ComponentT: Component> Renderable for ComponentT {
    #[inline]
    fn update(&mut self, message: DynamicMessage) -> ShouldRender {
        <Self as Component>::update(
            self,
            *message
                .0
                .downcast()
                .expect("Incorrect `Message` type when downcasting"),
        )
    }

    #[inline]
    fn change(&mut self, properties: DynamicProperties) -> ShouldRender {
        <Self as Component>::change(
            self,
            *properties
                .0
                .downcast()
                .expect("Incorrect `Properties` type when downcasting"),
        )
    }

    #[inline]
    fn resize(&mut self, frame: Rect) -> ShouldRender {
        <Self as Component>::resize(self, frame)
    }

    #[inline]
    fn view(&self) -> Layout {
        <Self as Component>::view(self)
    }

    #[inline]
    fn bindings(&self, bindings: &mut DynamicBindings) {
        bindings.typed(|bindings| <Self as Component>::bindings(self, bindings));
    }

    fn notify_binding_queries(&self, bindings: &[Option<NamedBindingQuery>], keys: &[KeyEvent]) {
        <Self as Component>::notify_binding_queries(self, bindings, keys);
    }

    #[inline]
    fn run_command(
        &self,
        bindings: &DynamicBindings,
        command_id: CommandId,
        keys: &[KeyEvent],
    ) -> Option<DynamicMessage> {
        bindings.execute_command(self, command_id, keys)
    }

    #[inline]
    fn tick(&self) -> Option<DynamicMessage> {
        <Self as Component>::tick(self).map(|message| DynamicMessage(Box::new(message)))
    }
}

pub(crate) trait Template {
    fn key(&self) -> Option<ComponentKey>;

    fn component_type_id(&self) -> TypeId;

    fn generate_id(&self, id: u64) -> ComponentId;

    fn create(
        &mut self,
        id: ComponentId,
        frame: Rect,
        sender: Box<dyn MessageSender>,
    ) -> (Box<dyn Renderable + 'static>, DynamicBindings);

    fn dynamic_properties(&mut self) -> DynamicProperties;
}

pub(crate) struct ComponentDef<ComponentT: Component> {
    pub key: Option<ComponentKey>,
    pub properties: Option<ComponentT::Properties>,
}

impl<ComponentT: Component> ComponentDef<ComponentT> {
    pub(crate) fn new(key: Option<ComponentKey>, properties: ComponentT::Properties) -> Self {
        Self {
            key,
            properties: properties.into(),
        }
    }

    fn properties_unwrap(&mut self) -> ComponentT::Properties {
        let mut properties = None;
        std::mem::swap(&mut properties, &mut self.properties);
        properties.expect("Already called a method that used the `Properties` value")
    }
}

impl<ComponentT: Component> Template for ComponentDef<ComponentT> {
    #[inline]
    fn key(&self) -> Option<ComponentKey> {
        self.key
    }

    #[inline]
    fn component_type_id(&self) -> TypeId {
        TypeId::of::<ComponentT>()
    }

    #[inline]
    fn generate_id(&self, position_hash: u64) -> ComponentId {
        ComponentId::new::<ComponentT>(position_hash)
    }

    #[inline]
    fn create(
        &mut self,
        component_id: ComponentId,
        frame: Rect,
        sender: Box<dyn MessageSender>,
    ) -> (Box<dyn Renderable>, DynamicBindings) {
        let link = ComponentLink::new(sender, component_id);
        (
            Box::new(ComponentT::create(self.properties_unwrap(), frame, link)),
            DynamicBindings::new::<ComponentT>(),
        )
    }

    #[inline]
    fn dynamic_properties(&mut self) -> DynamicProperties {
        DynamicProperties(Box::new(self.properties_unwrap()))
    }
}
