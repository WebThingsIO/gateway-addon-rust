use serde_json::Value;
pub use webthings_gateway_ipc_types::{Link, Property as PropertyDescription};

pub enum Type {
    Null,
    Boolean,
    Integer,
    Number,
    String,
}

impl ToString for Type {
    fn to_string(&self) -> String {
        match self {
            Type::Null => "null",
            Type::Boolean => "boolean",
            Type::Integer => "integer",
            Type::Number => "number",
            Type::String => "string",
        }
        .to_owned()
    }
}

#[derive(Debug)]
pub enum AtType {
    Alarm,
    AirQualitySensor,
    BarometricPressureSensor,
    BinarySensor,
    Camera,
    ColorControl,
    ColorSensor,
    DoorSensor,
    EnergyMonitor,
    HumiditySensor,
    LeakSensor,
    Light,
    Lock,
    MotionSensor,
    MultiLevelSensor,
    MultiLevelSwitch,
    OnOffSwitch,
    PushButton,
    SmartPlug,
    SmokeSensor,
    TemperatureSensor,
    Thermostat,
    VideoCamera,
    AlarmProperty,
    BarometricPressureProperty,
    BooleanProperty,
    BrightnessProperty,
    ColorModeProperty,
    ColorProperty,
    ColorTemperatureProperty,
    ConcentrationProperty,
    CurrentProperty,
    DensityProperty,
    FrequencyProperty,
    HeatingCoolingProperty,
    HumidityProperty,
    ImageProperty,
    InstantaneousPowerFactorProperty,
    InstantaneousPowerProperty,
    LeakProperty,
    LevelProperty,
    LockedProperty,
    MotionProperty,
    OnOffProperty,
    OpenProperty,
    PushedProperty,
    SmokeProperty,
    TargetTemperatureProperty,
    TemperatureProperty,
    ThermostatModeProperty,
    VideoProperty,
    VoltageProperty,
    FadeAction,
    LockAction,
    ToggleAction,
    UnlockAction,
    AlarmEvent,
    DoublePressedEvent,
    LongPressedEvent,
    OverheatedEvent,
    PressedEvent,
}

impl ToString for AtType {
    fn to_string(&self) -> String {
        format!("{:?}", self)
    }
}

pub trait PropertyDescriptionBuilder {
    fn at_type(self, at_type: AtType) -> Self;
    fn description<S: Into<String>>(self, description: S) -> Self;
    fn enum_(self, enum_: Vec<Value>) -> Self;
    fn links(self, links: Vec<Link>) -> Self;
    fn link(self, links: Link) -> Self;
    fn maximum(self, maximum: f64) -> Self;
    fn minimum(self, minimum: f64) -> Self;
    fn multiple_of(self, multiple_of: f64) -> Self;
    fn name<S: Into<String>>(self, name: S) -> Self;
    fn read_only(self, read_only: bool) -> Self;
    fn title<S: Into<String>>(self, title: S) -> Self;
    fn type_(self, type_: Type) -> Self;
    fn unit<S: Into<String>>(self, unit: S) -> Self;
    fn value<V: Into<Value>>(self, value: V) -> Self;
    fn visible(self, visible: bool) -> Self;
    fn default() -> Self;
}

impl PropertyDescriptionBuilder for PropertyDescription {
    fn at_type(mut self, at_type: AtType) -> Self {
        self.at_type = Some(at_type.to_string());
        self
    }

    fn description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }

    fn enum_(mut self, enum_: Vec<Value>) -> Self {
        self.enum_ = Some(enum_);
        self
    }

    fn links(mut self, links: Vec<Link>) -> Self {
        self.links = Some(links);
        self
    }

    fn link(mut self, link: Link) -> Self {
        match self.links {
            None => self.links = Some(vec![link]),
            Some(ref mut links) => links.push(link),
        };
        self
    }

    fn maximum(mut self, maximum: f64) -> Self {
        self.maximum = Some(maximum);
        self
    }

    fn minimum(mut self, minimum: f64) -> Self {
        self.minimum = Some(minimum);
        self
    }

    fn multiple_of(mut self, multiple_of: f64) -> Self {
        self.multiple_of = Some(multiple_of);
        self
    }

    fn name<S: Into<String>>(mut self, name: S) -> Self {
        self.name = Some(name.into());
        self
    }

    fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = Some(read_only);
        self
    }

    fn title<S: Into<String>>(mut self, title: S) -> Self {
        self.title = Some(title.into());
        self
    }

    fn type_(mut self, type_: Type) -> Self {
        self.type_ = type_.to_string();
        self
    }

    fn unit<S: Into<String>>(mut self, unit: S) -> Self {
        self.unit = Some(unit.into());
        self
    }

    fn value<V: Into<Value>>(mut self, value: V) -> Self {
        self.value = Some(value.into());
        self
    }

    fn visible(mut self, visible: bool) -> Self {
        self.visible = Some(visible);
        self
    }

    fn default() -> Self {
        Self {
            at_type: None,
            description: None,
            enum_: None,
            links: None,
            maximum: None,
            minimum: None,
            multiple_of: None,
            name: None,
            read_only: None,
            title: None,
            type_: Type::Null.to_string(),
            unit: None,
            value: None,
            visible: None,
        }
    }
}
