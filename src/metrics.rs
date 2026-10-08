use std::collections::HashMap;

use anyhow::{Result, bail};
use serde_json::{Map, Value};

use crate::switchbot::Device;

const FAMILIES: [(&str, &str, &str); 7] = [
    ("battery", "battery", "Battery level"),
    ("humidity", "humidity", "Humidity"),
    ("temperature", "temperature", "Temperature"),
    ("CO2", "co2", "CO2"),
    ("voltage", "voltage", "Voltage"),
    ("weight", "weight", "Weight"),
    ("electricCurrent", "electric_current", "ElectricCurrent"),
];

#[derive(Default)]
pub struct Metrics {
    names: HashMap<String, String>,
    samples: [Vec<(String, String)>; 7],
}

impl Metrics {
    pub fn add(&mut self, device: &Device, status: &Map<String, Value>) -> Result<()> {
        self.names
            .insert(device.device_id.clone(), device.device_name.clone());
        for (index, (key, _, _)) in FAMILIES.iter().enumerate() {
            if let Some(value) = status.get(*key) {
                if !value.is_number() {
                    bail!("SwitchBot metric {key} is not numeric");
                }
                let sample = (device.device_id.clone(), value.to_string());
                // Python の dict と同じく、重複IDは最初の順序で値を更新する。
                if let Some(existing) = self.samples[index]
                    .iter_mut()
                    .find(|(id, _)| id == &device.device_id)
                {
                    *existing = sample;
                } else {
                    self.samples[index].push(sample);
                }
            }
        }
        Ok(())
    }

    pub fn render(&self) -> String {
        let mut lines = Vec::new();
        for (index, (_, suffix, help)) in FAMILIES.iter().enumerate() {
            let name = format!("switchbot_device_{suffix}");
            lines.push(format!("# HELP {name} SwitchBot {help}"));
            lines.push(format!("# TYPE {name} gauge"));
            for (id, value) in &self.samples[index] {
                lines.push(format!(
                    "{name}{{device_id=\"{}\",device_name=\"{}\"}} {value}",
                    escape_label(id),
                    escape_label(&self.names[id]),
                ));
            }
        }
        lines.join("\n")
    }
}

pub fn escape_label(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('"', "\\\"")
}
