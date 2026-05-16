use unbill_model::{NodeId, SecretKey};

/// Extension trait on `NodeId` to convert to the network-layer type.
pub trait NodeIdExt {
    fn to_endpoint_id(&self) -> Result<iroh::EndpointId, iroh::KeyParsingError>;
}

/// Extension trait on `iroh::EndpointId` to convert to the model-layer type.
pub trait EndpointIdExt {
    fn to_node_id(&self) -> NodeId;
}

impl NodeIdExt for NodeId {
    fn to_endpoint_id(&self) -> Result<iroh::EndpointId, iroh::KeyParsingError> {
        self.as_str().parse()
    }
}

impl EndpointIdExt for iroh::EndpointId {
    fn to_node_id(&self) -> NodeId {
        NodeId::new(self.to_string())
    }
}

/// Extension trait on `SecretKey` (model) to convert to the iroh network type.
pub trait SecretKeyExt {
    fn to_iroh_key(&self) -> iroh::SecretKey;
}

/// Extension trait on `iroh::SecretKey` to convert to the model type.
pub trait IrohSecretKeyExt {
    fn to_model_key(&self) -> SecretKey;
}

impl SecretKeyExt for SecretKey {
    fn to_iroh_key(&self) -> iroh::SecretKey {
        iroh::SecretKey::from(*self.as_bytes())
    }
}

impl IrohSecretKeyExt for iroh::SecretKey {
    fn to_model_key(&self) -> SecretKey {
        SecretKey::from_bytes(self.to_bytes())
    }
}
