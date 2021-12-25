use gateway_addon_rust::{adapter, device, Adapter, Device};

#[adapter]
struct NamedExampleAdapter {}

impl Adapter for NamedExampleAdapter {}

#[adapter]
struct UnnamedExampleAdapter();

impl Adapter for UnnamedExampleAdapter {}

#[adapter]
struct UnitExampleAdapter;

impl Adapter for UnitExampleAdapter {}

#[device]
struct NamedExampleDevice {}

impl Device for NamedExampleDevice {}

#[device]
struct UnnamedExampleDevice();

impl Device for UnnamedExampleDevice {}

#[device]
struct UnitExampleDevice;

impl Device for UnitExampleDevice {}
