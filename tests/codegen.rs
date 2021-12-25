use gateway_addon_rust::{adapter, Adapter};

#[adapter]
struct NamedExampleAdapter {}

impl Adapter for NamedExampleAdapter {}

#[adapter]
struct UnnamedExampleAdapter();

impl Adapter for UnnamedExampleAdapter {}

#[adapter]
struct UnitExampleAdapter;

impl Adapter for UnitExampleAdapter {}
