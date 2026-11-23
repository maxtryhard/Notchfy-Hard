//! Avisos de bateria na ilha: carregador ligado/desligado, bateria fraca e
//! carga completa. Tudo vem do próprio sistema, sem permissão e sem rede.
//! Em computador sem bateria nada aparece.

use std::{thread, time::Duration};

use serde::Serialize;

pub const BATTERY_EVENT: &str = "island-battery";

/// O nível muda devagar; ler a cada 20 s basta e não pesa.
const POLL: Duration = Duration::from_secs(20);
const LOW_LEVELS: [u8; 2] = [20, 10];
/// Acima disto o aviso de bateria fraca volta a valer (evita repetir quando o
/// nível oscila em volta de 20%).
const LOW_RESET: u8 = 25;
const FULL_RESET: u8 = 95;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Reading {
    pub percent: u8,
    /// Ligado na tomada (carregando ou já cheio).
    pub plugged: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Alert {
    Plugged { percent: u8 },
    Unplugged { percent: u8 },
    Low { percent: u8 },
    Full,
}

/// Decide o que avisar comparando cada leitura com a anterior.
#[derive(Default)]
pub struct Tracker {
    last: Option<Reading>,
    /// Menor nível de "bateria fraca" já avisado nesta descarga.
    low_warned: Option<u8>,
    full_warned: bool,
}

impl Tracker {
    pub fn next(&mut self, now: Reading) -> Option<Alert> {
        let Some(last) = self.last.replace(now) else {
            // Primeira leitura: abrir o app não é motivo para avisar nada.
            self.full_warned = now.plugged && now.percent >= 100;
            return None;
        };
        if now.percent > LOW_RESET || now.plugged {
            self.low_warned = None;
        }
        if !now.plugged || now.percent < FULL_RESET {
            self.full_warned = false;
        }
        if now.plugged != last.plugged {
            let percent = now.percent;
            return Some(if now.plugged {
                Alert::Plugged { percent }
            } else {
                Alert::Unplugged { percent }
            });
        }
        if now.plugged && now.percent >= 100 && !self.full_warned {
            self.full_warned = true;
            return Some(Alert::Full);
        }
        if !now.plugged {
            let level = LOW_LEVELS.into_iter().filter(|&l| now.percent <= l).min()?;
            if self.low_warned.is_none_or(|w| level < w) {
                self.low_warned = Some(level);
                return Some(Alert::Low {
                    percent: now.percent,
                });
            }
        }
        None
    }
}

/// Lê a bateria em uma thread própria e chama `on_alert` quando há algo a
/// avisar. Sem bateria, a leitura dá `None` e nada acontece.
pub fn watch(on_alert: impl Fn(Alert) + Send + 'static) {
    thread::spawn(move || {
        let mut tracker = Tracker::default();
        loop {
            if let Some(alert) = imp::read().and_then(|r| tracker.next(r)) {
                on_alert(alert);
            }
            thread::sleep(POLL);
        }
    });
}

/// Saída do `pmset -g batt`. A primeira linha diz a fonte de energia e a
/// linha da bateria interna traz a porcentagem.
#[cfg(any(target_os = "macos", test))]
fn parse_pmset(text: &str) -> Option<Reading> {
    let plugged = text.lines().next()?.contains("'AC Power'");
    let line = text.lines().find(|l| l.contains("InternalBattery"))?;
    let percent = line.split(['\t', ' ']).find_map(|w| w.strip_suffix("%;"))?;
    Some(Reading {
        percent: percent.parse::<u8>().ok()?.min(100),
        plugged,
    })
}

/// Arquivos de `/sys/class/power_supply/<item>/`. `scope = Device` marca
/// baterias de mouse e fones, que não são a do computador.
#[cfg(any(target_os = "linux", test))]
fn parse_sysfs(kind: &str, scope: Option<&str>, capacity: &str, status: &str) -> Option<Reading> {
    if kind.trim() != "Battery" || scope.is_some_and(|s| s.trim() == "Device") {
        return None;
    }
    Some(Reading {
        percent: capacity.trim().parse::<u8>().ok()?.min(100),
        plugged: status.trim() != "Discharging",
    })
}

#[cfg(target_os = "linux")]
mod imp {
    use std::{fs, path::Path};

    fn file(dir: &Path, name: &str) -> Option<String> {
        // Arquivos do kernel com poucos bytes; não há o que limitar.
        fs::read_to_string(dir.join(name)).ok()
    }

    pub fn read() -> Option<super::Reading> {
        fs::read_dir("/sys/class/power_supply")
            .ok()?
            .flatten()
            .find_map(|entry| {
                let dir = entry.path();
                super::parse_sysfs(
                    &file(&dir, "type")?,
                    file(&dir, "scope").as_deref(),
                    &file(&dir, "capacity")?,
                    &file(&dir, "status")?,
                )
            })
    }
}

#[cfg(windows)]
mod imp {
    use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};

    /// `BatteryFlag` com este bit: o computador não tem bateria.
    const NO_BATTERY: u8 = 128;
    const UNKNOWN_PERCENT: u8 = 255;

    pub fn read() -> Option<super::Reading> {
        let mut status = SYSTEM_POWER_STATUS::default();
        // SAFETY: a função só preenche a struct que passamos, que vive até o fim.
        unsafe { GetSystemPowerStatus(&mut status) }.ok()?;
        if status.BatteryFlag & NO_BATTERY != 0 || status.BatteryLifePercent == UNKNOWN_PERCENT {
            return None;
        }
        Some(super::Reading {
            percent: status.BatteryLifePercent.min(100),
            plugged: status.ACLineStatus == 1,
        })
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use std::process::Command;

    pub fn read() -> Option<super::Reading> {
        let out = Command::new("pmset").arg("-g").arg("batt").output().ok()?;
        super::parse_pmset(&String::from_utf8_lossy(&out.stdout))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(percent: u8, plugged: bool) -> Reading {
        Reading { percent, plugged }
    }

    fn run(readings: &[Reading]) -> Vec<Alert> {
        let mut t = Tracker::default();
        readings.iter().filter_map(|&x| t.next(x)).collect()
    }

    #[test]
    fn first_reading_is_silent() {
        assert_eq!(run(&[r(5, false)]), vec![]);
        assert_eq!(run(&[r(100, true), r(100, true)]), vec![]);
    }

    #[test]
    fn reports_plug_changes() {
        assert_eq!(
            run(&[r(50, false), r(50, true), r(51, false)]),
            vec![
                Alert::Plugged { percent: 50 },
                Alert::Unplugged { percent: 51 }
            ]
        );
    }

    #[test]
    fn warns_low_once_per_level() {
        let alerts = run(&[
            r(22, false),
            r(20, false),
            r(19, false),
            r(21, false),
            r(20, false),
            r(10, false),
            r(9, false),
        ]);
        assert_eq!(
            alerts,
            vec![Alert::Low { percent: 20 }, Alert::Low { percent: 10 }]
        );
    }

    #[test]
    fn low_warning_rearms_after_charging() {
        let alerts = run(&[
            r(21, false),
            r(20, false),
            r(20, true),
            r(20, false),
            r(19, false),
        ]);
        assert_eq!(
            alerts,
            vec![
                Alert::Low { percent: 20 },
                Alert::Plugged { percent: 20 },
                Alert::Unplugged { percent: 20 },
                Alert::Low { percent: 19 },
            ]
        );
    }

    #[test]
    fn warns_full_once() {
        let alerts = run(&[
            r(98, true),
            r(100, true),
            r(100, true),
            r(99, true),
            r(100, true),
        ]);
        assert_eq!(alerts, vec![Alert::Full]);
    }

    #[test]
    fn reads_pmset_output() {
        let charging = "Now drawing from 'AC Power'\n -InternalBattery-0 (id=4653155)\t87%; charging; 0:42 remaining present: true\n";
        assert_eq!(parse_pmset(charging), Some(r(87, true)));
        let battery = "Now drawing from 'Battery Power'\n -InternalBattery-0 (id=4653155)\t100%; discharging; 9:12 remaining present: true\n";
        assert_eq!(parse_pmset(battery), Some(r(100, false)));
        // Mac de mesa: só a primeira linha.
        assert_eq!(parse_pmset("Now drawing from 'AC Power'\n"), None);
    }

    #[test]
    fn reads_sysfs_files() {
        assert_eq!(
            parse_sysfs("Battery\n", Some("System\n"), "64\n", "Discharging\n"),
            Some(r(64, false))
        );
        assert_eq!(
            parse_sysfs("Battery\n", None, "100\n", "Full\n"),
            Some(r(100, true))
        );
        assert_eq!(
            parse_sysfs("Battery\n", None, "80\n", "Not charging\n"),
            Some(r(80, true))
        );
        assert_eq!(parse_sysfs("Mains\n", None, "1\n", "\n"), None);
        assert_eq!(
            parse_sysfs("Battery\n", Some("Device\n"), "40\n", "Discharging\n"),
            None
        );
    }
}
