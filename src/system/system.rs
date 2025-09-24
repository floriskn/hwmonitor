use std::sync::Arc;

use crate::system::{
    cpu::cpu::{gather_cpus, Cpu},
    kernal_driver::{DriverBuilder, KernelDriver},
};

#[derive(Debug)]
pub struct System {
    driver: Arc<KernelDriver>,
    pub cpu: Option<Vec<Cpu>>,
}

impl System {
    pub fn builder() -> SystemBuilder {
        SystemBuilder::default()
    }

    // Internal constructor used by builder
    fn new(driver: Arc<KernelDriver>, cpu: Option<Vec<Cpu>>) -> Self {
        Self { driver, cpu }
    }

    /// Explicit close
    pub fn close(self) -> Result<(), String> {
        // Force close/uninstall through RefCell
        self.driver.close()?;
        self.driver.uninstall()?;

        println!("uninstalled (forced)");
        Ok(())
    }
}

// Builder struct
#[derive(Default)]
pub struct SystemBuilder {
    enable_cpu: bool,
    // future: enable_gpu, enable_ram, etc.
}

impl SystemBuilder {
    pub fn cpu(mut self) -> Self {
        self.enable_cpu = true;
        self
    }

    pub fn build(self) -> Result<System, String> {
        // Select the driver binary based on architecture
        let driver_bin: &[u8] = if cfg!(target_arch = "x86_64") {
            include_bytes!("../../resources/WinRing0x64.sys")
        } else {
            include_bytes!("../../resources/WinRing0.sys")
        };

        // Create driver
        let mut driver = DriverBuilder::new()
            .set_device_description("Hw Monitor Driver")
            .set_device_id("WinRing0_1_2_0")
            .set_driver_bin(driver_bin.to_vec())
            .build()?;
        // driver.close()?;
        // driver.uninstall()?;

        // return Err("t".into());

        // Install
        driver.install()?;

        // Open driver
        if let Err(e) = driver.open() {
            let _ = driver.uninstall();
            return Err(format!("Failed to open driver: {}", e));
        }

        // Wrap in Rc after successful open
        let driver_rc = Arc::new(driver);

        // Centralized helper to initialize a subsystem
        fn init_subsystem<T>(
            driver: &Arc<KernelDriver>,
            enabled: bool,
            f: impl Fn(&Arc<KernelDriver>) -> Result<T, String>,
        ) -> Result<Option<T>, String> {
            if enabled {
                match f(driver) {
                    Ok(sub) => Ok(Some(sub)),
                    Err(e) => {
                        // Cleanup driver on failure
                        let _ = Arc::get_mut(&mut driver.clone()).map(|d| {
                            let _ = d.close();
                            let _ = d.uninstall();

                            println!("Failure cleanup")
                        });
                        Err(e)
                    }
                }
            } else {
                Ok(None)
            }
        }

        // Initialize subsystems
        let cpu = init_subsystem(&driver_rc, self.enable_cpu, |drv| gather_cpus(drv))?;

        Ok(System::new(driver_rc, cpu))
    }
}
