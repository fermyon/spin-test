use std::{cell::RefCell, rc::Rc};

/// A composition of components
#[derive(Default, Debug)]
pub struct Composition {
    graph: Rc<RefCell<wac_graph::CompositionGraph>>,
}

impl Composition {
    /// Create a new composition
    pub fn new() -> Self {
        Self::default()
    }

    /// Instantiate a component in the composition
    ///
    /// This automatically registers the package with the composition.
    pub fn instantiate<'a>(
        &self,
        name: &str,
        bytes: &[u8],
        arguments: impl IntoIterator<Item = (&'a str, &'a dyn InstantiationArg)> + 'a,
    ) -> anyhow::Result<Instance> {
        let package = self.register_package(name, bytes)?;
        package.instantiate(arguments)
    }

    pub fn register_package(&self, name: &str, bytes: &[u8]) -> anyhow::Result<Package> {
        let package = wac_graph::types::Package::from_bytes(
            name,
            None,
            bytes.to_owned(),
            self.graph.borrow_mut().types_mut(),
        )?;
        let package = self.graph.borrow_mut().register_package(package)?;
        Ok(Package {
            graph: self.graph.clone(),
            id: package,
            name: name.to_owned(),
        })
    }

    /// Import an instance into the composition
    pub fn import_instance(&self, name: &str, instance: InstanceItem) -> anyhow::Result<Instance> {
        let node_id = self.graph.borrow_mut().import(
            name,
            wac_graph::types::ItemKind::Instance(instance.instance_id),
        )?;
        Ok(Instance {
            graph: self.graph.clone(),
            id: node_id,
            name: name.to_owned(),
        })
    }

    /// Export an instance's export with a given name from the composition
    pub fn export(&self, export: InstanceExport, name: &str) -> anyhow::Result<()> {
        Ok(self.graph.borrow_mut().export(export.id, name)?)
    }

    /// Encode the composition into a component binary
    pub fn encode(&self, validate: bool) -> anyhow::Result<Vec<u8>> {
        Ok(self.graph.borrow_mut().encode(wac_graph::EncodeOptions {
            validate,
            ..Default::default()
        })?)
    }
}

/// An instance of a component in a composition
#[derive(Clone)]
pub struct Instance {
    graph: Rc<RefCell<wac_graph::CompositionGraph>>,
    id: wac_graph::NodeId,
    name: String,
}

impl Instance {
    /// Export a node from the instance
    ///
    /// Returns `None` if no export exists with the given name
    pub fn export(&self, name: &str) -> anyhow::Result<Option<InstanceExport>> {
        let node = self
            .graph
            .borrow_mut()
            .alias_instance_export(self.id, name)
            .map(Some)
            .or_else(|e| match e {
                wac_graph::AliasError::InstanceMissingExport { .. } => Ok(None),
                e => Err(e),
            })?;

        Ok(node.map(|node_id| {
            let node = &RefCell::borrow(&self.graph)[node_id];
            InstanceExport {
                graph: self.graph.clone(),
                id: node_id,
                kind: node.item_kind(),
                name: name.to_owned(),
            }
        }))
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

/// A package in a composition
pub struct Package {
    graph: Rc<RefCell<wac_graph::CompositionGraph>>,
    id: wac_graph::PackageId,
    name: String,
}

impl Package {
    /// Instantiate the package with the given arguments
    pub fn instantiate<'a>(
        &self,
        arguments: impl IntoIterator<Item = (&'a str, &'a dyn InstantiationArg)>,
    ) -> anyhow::Result<Instance> {
        let instance = self.graph.borrow_mut().instantiate(self.id);
        for (arg_name, arg) in arguments {
            match self
                .graph
                .borrow_mut()
                .set_instantiation_argument(instance, arg_name, arg.id())
            {
                // Don't error if we try to pass an invalid argument
                Ok(_) | Err(wac_graph::InstantiationArgumentError::InvalidArgumentName { .. }) => {}
                Err(e) => return Err(e.into()),
            }
        }
        Ok(Instance {
            graph: self.graph.clone(),
            id: instance,
            name: self.name.clone(),
        })
    }

    /// Get an exported item from the package
    pub fn get_export(&self, export_name: &str) -> Option<PackageItem> {
        let graph = self.graph.borrow_mut();
        let package = &graph[self.id];
        let package_world = &graph.types()[package.ty()];
        let kind = package_world.exports.get(export_name).cloned()?;
        Some(PackageItem {
            graph: self.graph.clone(),
            kind,
        })
    }
}

/// A component model item in a package
///
/// An item is anything a component contains (e.g., another component, a function, a type, an instance, etc.)
pub struct PackageItem {
    graph: Rc<RefCell<wac_graph::CompositionGraph>>,
    kind: wac_graph::types::ItemKind,
}

impl PackageItem {
    /// View the item as a component if it is one.
    ///
    /// Component types will be promoted into component items.
    pub fn as_component(&self) -> Option<ComponentItem> {
        if let wac_graph::types::ItemKind::Component(world_id) = self.kind.promote() {
            Some(ComponentItem {
                graph: self.graph.clone(),
                world_id,
            })
        } else {
            None
        }
    }

    /// View the item as an instance if it is one.
    ///
    /// Instance types will be promoted into instance items.
    pub fn as_instance(&self) -> Option<InstanceItem> {
        if let wac_graph::types::ItemKind::Instance(instance_id) = self.kind.promote() {
            Some(InstanceItem { instance_id })
        } else {
            None
        }
    }
}

/// A component item inside of a component.
pub struct ComponentItem {
    graph: Rc<RefCell<wac_graph::CompositionGraph>>,
    world_id: wac_graph::types::WorldId,
}

impl ComponentItem {
    /// Get an exported item from the component.
    ///
    /// Types will be promoted into package items.
    pub fn get_export(&self, export_name: &str) -> Option<PackageItem> {
        let graph = self.graph.borrow_mut();
        let world = &graph.types()[self.world_id];
        let kind = world.exports.get(export_name)?.promote();
        Some(PackageItem {
            graph: self.graph.clone(),
            kind,
        })
    }
}

/// An instance item inside of a component
pub struct InstanceItem {
    instance_id: wac_graph::types::InterfaceId,
}

/// An export from an instantiated instance
#[derive(Clone)]
pub struct InstanceExport {
    name: String,
    graph: Rc<RefCell<wac_graph::CompositionGraph>>,
    id: wac_graph::NodeId,
    kind: wac_graph::types::ItemKind,
}

impl InstanceExport {
    /// View the export as an instance if it is one.
    pub fn as_instance(&self) -> Option<Instance> {
        if let wac_graph::types::ItemKind::Instance(_) = self.kind {
            Some(Instance {
                graph: self.graph.clone(),
                id: self.id,
                name: self.name.clone(),
            })
        } else {
            None
        }
    }
}

/// An argument to an instantiation
pub trait InstantiationArg {
    fn id(&self) -> wac_graph::NodeId;
}

impl InstantiationArg for InstanceExport {
    fn id(&self) -> wac_graph::NodeId {
        self.id
    }
}

impl InstantiationArg for Instance {
    fn id(&self) -> wac_graph::NodeId {
        self.id
    }
}
