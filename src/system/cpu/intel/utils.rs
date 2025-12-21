use super::{micro_architecture::MicroArchitecture, tj_max::CpuTJMax};

pub fn get_cpu_tjmax_info(
    family_id: u8,
    model_id: u8,
    stepping_id: u8,
    core_count: usize,
) -> (CpuTJMax, MicroArchitecture) {
    match family_id {
        0x06 => match model_id {
            0x0F => {
                // Intel Core 2 (65nm)
                let tj = match stepping_id {
                    // B2
                    0x06 => match core_count {
                        2 => CpuTJMax::Static(90.0),
                        4 => CpuTJMax::Static(100.0),
                        _ => CpuTJMax::Static(95.0),
                    },
                    // G0
                    0x0B => CpuTJMax::Static(100.0),
                    // M0
                    0x0D => CpuTJMax::Static(95.0),
                    _ => CpuTJMax::Static(95.0),
                };
                (tj, MicroArchitecture::Core)
            }

            // Intel Core 2 (45nm)
            0x17 => (CpuTJMax::Static(100.0), MicroArchitecture::Core),

            0x1C => {
                // Intel Atom (45nm)
                let tj = match stepping_id {
                    // C0
                    0x02 => CpuTJMax::Static(90.0),
                    // A0, B0
                    0x0A => CpuTJMax::Static(100.0),
                    _ => CpuTJMax::Static(90.0),
                };
                (tj, MicroArchitecture::Atom)
            }

            /* Nehalem
             * 0x1A: Core i7 LGA1366 (45nm)
             * 0x1E: Core i5/i7 LGA1156 (45nm)
             * 0x1F: Core i5/i7
             * 0x25: Core i3/i5/i7 LGA1156 (32nm)
             * 0x2C: Core i7 LGA1366 6C (32nm)
             * 0x2E: Xeon 7500 (45nm)
             * 0x2F: Xeon (32nm)
             */
            0x1A | 0x1E | 0x1F | 0x25 | 0x2C | 0x2E | 0x2F => {
                (CpuTJMax::MSR, MicroArchitecture::Nehalem)
            }

            /* Sandy Bridge */
            0x2A | 0x2D => (CpuTJMax::MSR, MicroArchitecture::SandyBridge),

            /* Ivy Bridge */
            0x3A | 0x3E => (CpuTJMax::MSR, MicroArchitecture::IvyBridge),

            /* Haswell */
            0x3C | 0x3F | 0x45 | 0x46 => (CpuTJMax::MSR, MicroArchitecture::Haswell),

            /* Broadwell */
            0x3D | 0x47 | 0x4F | 0x56 => (CpuTJMax::MSR, MicroArchitecture::Broadwell),

            /* Atom */
            0x36 => (CpuTJMax::MSR, MicroArchitecture::Atom),

            /* Silvermont */
            0x37 | 0x4A | 0x4D | 0x5A | 0x5D => (CpuTJMax::MSR, MicroArchitecture::Silvermont),

            /* Skylake */
            0x4E | 0x5E | 0x55 => (CpuTJMax::MSR, MicroArchitecture::Skylake),

            /* Airmont */
            0x4C => (CpuTJMax::MSR, MicroArchitecture::Airmont),

            /* Kaby / Coffee Lake */
            0x8E | 0x9E => (CpuTJMax::MSR, MicroArchitecture::KabyLake),

            /* Goldmont */
            0x5C | 0x5F => (CpuTJMax::MSR, MicroArchitecture::Goldmont),

            /* Goldmont Plus */
            0x7A => (CpuTJMax::MSR, MicroArchitecture::GoldmontPlus),

            /* Cannon Lake */
            0x66 => (CpuTJMax::MSR, MicroArchitecture::CannonLake),

            /* Ice Lake */
            0x7D | 0x7E | 0x6A | 0x6C => (CpuTJMax::MSR, MicroArchitecture::IceLake),

            /* Comet Lake */
            0xA5 | 0xA6 => (CpuTJMax::MSR, MicroArchitecture::CometLake),

            /* Tremont */
            0x86 => (CpuTJMax::MSR, MicroArchitecture::Tremont),

            /* Tiger Lake */
            0x8C | 0x8D => (CpuTJMax::MSR, MicroArchitecture::TigerLake),

            /* Alder Lake */
            0x97 | 0x9A | 0xBE => (CpuTJMax::MSR, MicroArchitecture::AlderLake),

            /* Raptor Lake */
            0xB7 | 0xBA | 0xBF => (CpuTJMax::MSR, MicroArchitecture::RaptorLake),

            /* Meteor Lake */
            0xAC | 0xAA => (CpuTJMax::MSR, MicroArchitecture::MeteorLake),

            /* Jasper Lake */
            0x9C => (CpuTJMax::MSR, MicroArchitecture::JasperLake),

            /* Rocket Lake */
            0xA7 => (CpuTJMax::MSR, MicroArchitecture::RocketLake),

            /* Arrow Lake */
            0xC5 | 0xC6 => (CpuTJMax::MSR, MicroArchitecture::ArrowLake),

            /* Lunar Lake */
            0xBD => (CpuTJMax::MSR, MicroArchitecture::LunarLake),

            /* Sapphire Rapids */
            0x8F => (CpuTJMax::MSR, MicroArchitecture::SapphireRapids),

            /* Elkhart Lake */
            0x96 => (CpuTJMax::MSR, MicroArchitecture::ElkhartLake),

            _ => (CpuTJMax::Static(100.0), MicroArchitecture::Unknown),
        },

        0x0F => match model_id {
            /* NetBurst
             * 0x00: Pentium 4 (180nm)
             * 0x01: Pentium 4 (130nm)
             * 0x02: Pentium 4 (130nm)
             * 0x03: Pentium 4, Celeron D (90nm)
             * 0x04: Pentium 4, Pentium D, Celeron D (90nm)
             * 0x06: Pentium 4, Pentium D, Celeron D (65nm)
             */
            0x00 | 0x01 | 0x02 | 0x03 | 0x04 | 0x06 => {
                (CpuTJMax::Static(100.0), MicroArchitecture::NetBurst)
            }

            _ => (CpuTJMax::Static(100.0), MicroArchitecture::Unknown),
        },

        _ => (CpuTJMax::Static(100.0), MicroArchitecture::Unknown),
    }
}
