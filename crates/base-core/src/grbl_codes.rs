//! What the alarms (`ALARM:2`) and the errors (`error:20`) of GRBL 1.1 mean, in a sentence for the person at the machine.
//! The sentences are English; the programs translate them with `tr`.

pub struct Code {
    pub n: u32,
    pub text: &'static str,
}

pub const ALARMS: &[Code] = &[
    Code { n: 1, text: "A limit switch was hit. The position may be lost: home the machine again." },
    Code { n: 2, text: "The move goes beyond the travel of the machine. The position is kept; unlock to go on." },
    Code { n: 3, text: "Reset while moving. Steps may have been lost: home the machine again." },
    Code { n: 4, text: "Probe failed: the probe was already closed before the move. Check the wiring and the plate." },
    Code { n: 5, text: "Probe failed: it touched nothing within the distance. Check the plate and the fail distance." },
    Code { n: 6, text: "Homing failed: it was reset during the cycle." },
    Code { n: 7, text: "Homing failed: the safety door was opened during the cycle." },
    Code { n: 8, text: "Homing failed: the switch did not release when pulling off. Check the pull-off and the wiring." },
    Code { n: 9, text: "Homing failed: no limit switch was found. Check the switches and the wiring." },
];

pub const ERRORS: &[Code] = &[
    Code { n: 1, text: "A G-code word has no letter." },
    Code { n: 2, text: "A number is not valid or is missing." },
    Code { n: 3, text: "GRBL does not know this $ command." },
    Code { n: 4, text: "A negative value where a positive one is needed." },
    Code { n: 5, text: "Homing is not enabled in the settings ($22)." },
    Code { n: 6, text: "The step pulse time must be at least 3 µs." },
    Code { n: 7, text: "The settings could not be read; the defaults were restored." },
    Code { n: 8, text: "This $ command works only when the machine is idle." },
    Code { n: 9, text: "G-code is locked during an alarm or a jog. Unlock first." },
    Code { n: 10, text: "Soft limits need homing to be enabled too." },
    Code { n: 11, text: "The line is too long." },
    Code { n: 12, text: "The setting is above the step rate this GRBL supports." },
    Code { n: 13, text: "The safety door is open." },
    Code { n: 14, text: "The build info or start-up line is too long." },
    Code { n: 15, text: "The jog goes beyond the travel of the machine and was ignored." },
    Code { n: 16, text: "The jog command is not valid." },
    Code { n: 17, text: "Laser mode needs a PWM output." },
    Code { n: 20, text: "A G-code command is not supported or not valid." },
    Code { n: 21, text: "Two commands of the same group are in one line." },
    Code { n: 22, text: "No feed rate is set yet." },
    Code { n: 23, text: "A command needs a whole number." },
    Code { n: 24, text: "Two commands in the line both need axis words." },
    Code { n: 25, text: "A word is repeated in the line." },
    Code { n: 26, text: "A command needs axis words but the line has none." },
    Code { n: 27, text: "The line number is out of range." },
    Code { n: 28, text: "A command misses its P or L value." },
    Code { n: 29, text: "Only the work coordinate systems G54 to G59 are supported." },
    Code { n: 30, text: "G53 works only with G0 or G1." },
    Code { n: 31, text: "Axis words are left over while G80 is active." },
    Code { n: 32, text: "An arc has no axis words in its plane." },
    Code { n: 33, text: "The target of the move is not valid." },
    Code { n: 34, text: "The arc cannot be computed from its radius. Split it or use offsets." },
    Code { n: 35, text: "The arc misses its I, J or K offset." },
    Code { n: 36, text: "Some words are not used by any command." },
    Code { n: 37, text: "G43.1 can offset only the tool length axis." },
    Code { n: 38, text: "The tool number is too high." },
];

pub fn alarm(n: u32) -> Option<&'static str> {
    ALARMS.iter().find(|c| c.n == n).map(|c| c.text)
}

pub fn error(n: u32) -> Option<&'static str> {
    ERRORS.iter().find(|c| c.n == n).map(|c| c.text)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Alarm,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Explained {
    pub kind: Kind,
    pub code: u32,
    /// `None` for a code this table does not know.
    pub text: Option<&'static str>,
}

/// Read a line that GRBL sent: `ALARM:2` or `error:20` (also `[MSG:…]`-free lines only).
pub fn explain(line: &str) -> Option<Explained> {
    let l = line.trim();
    if let Some(n) = l.strip_prefix("ALARM:").and_then(|n| n.trim().parse::<u32>().ok()) {
        return Some(Explained { kind: Kind::Alarm, code: n, text: alarm(n) });
    }
    if let Some(n) = l.strip_prefix("error:").and_then(|n| n.trim().parse::<u32>().ok()) {
        return Some(Explained { kind: Kind::Error, code: n, text: error(n) });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_found() {
        assert_eq!(explain("ALARM:5").map(|e| (e.kind, e.code)), Some((Kind::Alarm, 5)));
        assert!(explain("error:20").unwrap().text.unwrap().contains("not supported"));
        assert_eq!(explain("error:99"), Some(Explained { kind: Kind::Error, code: 99, text: None }));
        assert!(explain("ok").is_none());
        assert!(explain("error:x").is_none());
        assert!(explain("[MSG:error:3]").is_none());
    }

    #[test]
    fn tables_are_in_order_without_doubles() {
        for t in [ALARMS, ERRORS] {
            assert!(t.windows(2).all(|w| w[0].n < w[1].n));
            assert!(t.iter().all(|c| !c.text.is_empty() && c.text.len() < 100));
        }
    }
}
