use std::{cell::RefCell, rc::Rc};

/// A composition of components
pub struct Composition {
    graph: Rc<RefCell<wac_graph::CompositionGraph>>,
}

impl Composition {
    /// Create a new composition
    pub fn new() -> Self {
        Self {
            graph: Rc::new(RefCell::new(wac_graph::CompositionGraph::new())),
        }
    }

    /// Instantiate a component in the composition
    pub fn instantiate<'a>(
        &self,
        name: &str,
        bytes: &[u8],
        arguments: impl IntoIterator<Item = (&'a str, Export)> + 'a,
    ) -> anyhow::Result<Instance> {
        let package = wac_graph::types::Package::from_bytes(
            name,
            None,
            bytes.to_owned(),
            self.graph.borrow_mut().types_mut(),
        )?;
        let package = self.graph.borrow_mut().register_package(package)?;
        let instance = self.graph.borrow_mut().instantiate(package);
        for (arg_name, arg) in arguments {
            match self
                .graph
                .borrow_mut()
                .set_instantiation_argument(instance, arg_name, arg.node)
            {
                // Don't error if we try to pass an invalid argument
                Ok(_) | Err(wac_graph::InstantiationArgumentError::InvalidArgumentName { .. }) => {}
                Err(e) => return Err(e.into()),
            }
        }
        Ok(Instance {
            graph: self.graph.clone(),
            node: instance,
        })
    }

    /// Export an instance's export with a given name
    pub fn export(&self, export: Export, name: &str) -> anyhow::Result<()> {
        Ok(self.graph.borrow_mut().export(export.node, name)?)
    }

    /// Encode the composition into a component binary
    pub fn encode(&self) -> anyhow::Result<Vec<u8>> {
        Ok(self
            .graph
            .borrow_mut()
            .encode(wac_graph::EncodeOptions::default())?)
    }
}

/// An instance of a component in a composition
pub struct Instance {
    graph: Rc<RefCell<wac_graph::CompositionGraph>>,
    node: wac_graph::NodeId,
}

impl Instance {
    /// Export a node from the instance
    ///
    /// Returns `None` if no export exists with the given name
    pub fn export(&self, name: &str) -> anyhow::Result<Option<Export>> {
        let node = self
            .graph
            .borrow_mut()
            .alias_instance_export(self.node, name)
            .map(Some)
            .or_else(|e| match e {
                wac_graph::AliasError::InstanceMissingExport { .. } => Ok(None),
                e => Err(e),
            })?;

        Ok(node.map(|node| Export { node }))
    }
}

/// An export from an instance
pub struct Export {
    node: wac_graph::NodeId,
}
