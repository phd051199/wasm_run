#[derive(Debug)]
pub struct WasiConfigNative {
    /// Whether to capture stdout.
    /// If this is true, you can use the [WasmInstance.stdout]
    /// getter to retrieve a stream of the module's stdout.
    pub capture_stdout: bool,
    /// Whether to capture stderr
    /// If this is true, you can use the [WasmInstance.stderr]
    /// getter to retrieve a stream of the module's stderr.
    pub capture_stderr: bool,
    // TODO: custom stdin
    /// Whether to inherit stdin from the host process.
    pub inherit_stdin: bool,
    /// Whether to inherit environment variables from the host process.
    pub inherit_env: bool,
    /// Whether to inherit the process arguments from the host process.
    pub inherit_args: bool,
    /// Custom process arguments to pass to the WASM module
    pub args: Vec<String>,
    /// Custom Environment variables to pass to the WASM module
    pub env: Vec<EnvVariable>,
    /// Custom preopened files to pass to the WASM module
    pub preopened_files: Vec<String>,
    /// Custom preopened directories to pass to the WASM module
    /// The module will be able to access and edit these directories
    pub preopened_dirs: Vec<PreopenedDir>,
}

#[derive(Debug)]
#[allow(non_camel_case_types)]
pub enum StdIOKind {
    stdout,
    stderr,
}

#[cfg(feature = "wasi")]
impl WasiConfigNative {
    pub fn to_wasi_ctx(&self) -> anyhow::Result<wasi_common::WasiCtx> {
        #[cfg(not(feature = "wasmtime"))]
        use wasmi_wasi::{ambient_authority, WasiCtxBuilder};
        #[cfg(feature = "wasmtime")]
        use wasmtime_wasi::{ambient_authority, WasiCtxBuilder};

        // add wasi to linker
        #[cfg(not(feature = "wasmtime"))]
        let mut wasi_builder = WasiCtxBuilder::new();
        #[cfg(feature = "wasmtime")]
        let mut wasi_builder = &mut WasiCtxBuilder::new();
        if self.inherit_args {
            wasi_builder = wasi_builder.inherit_args()?;
        }
        if self.inherit_env {
            wasi_builder = wasi_builder.inherit_env()?;
        }
        if self.inherit_stdin {
            wasi_builder = wasi_builder.inherit_stdin();
        }
        if !self.capture_stdout {
            wasi_builder = wasi_builder.inherit_stdout();
        }
        if !self.capture_stderr {
            wasi_builder = wasi_builder.inherit_stderr();
        }
        if !self.args.is_empty() {
            for value in &self.args {
                wasi_builder = wasi_builder.arg(value)?;
            }
        }
        if !self.env.is_empty() {
            for EnvVariable { name, value } in &self.env {
                wasi_builder = wasi_builder.env(name, value)?;
            }
        }
        if !self.preopened_dirs.is_empty() {
            for PreopenedDir {
                wasm_guest_path,
                host_path,
            } in &self.preopened_dirs
            {
                let dir = cap_std::fs::Dir::open_ambient_dir(host_path, ambient_authority())?;
                wasi_builder = wasi_builder.preopened_dir(dir, wasm_guest_path)?;
            }
        }

        Ok(wasi_builder.build())
    }
}

#[derive(Debug)]
pub struct EnvVariable {
    /// The name of the environment variable
    pub name: String,
    /// The value of the environment variable
    pub value: String,
}

/// A preopened directory that the WASM module will be able to access
#[derive(Debug)]
pub struct PreopenedDir {
    /// The path inside the WASM module.
    /// Should be "/" separated, if you are on windows, you will need to convert the path
    pub wasm_guest_path: String,
    /// The path on the host that the WASM module will be able to access
    /// and corresponds to the [wasm_guest_path]
    pub host_path: String,
}

pub struct WasmRuntimeFeatures {
    /// The name of the runtime.
    /// For example, "wasmi" or "wasmtime".
    pub name: String,
    /// The version of the runtime.
    /// For example, "0.31.0" or "14.0.4".
    pub version: String,
    /// Is `true` if the runtime is the one provided by the browser.
    pub is_browser: bool,
    /// The features supported by the runtime.
    pub supported_features: WasmFeatures,
    /// The default features of the runtime.
    /// If a feature is supported, but it is not enable by default,
    /// then it must be enabled manually, perhaps with [ModuleConfig],
    /// and it may be experimental.
    pub default_features: WasmFeatures,
}

impl Default for WasmRuntimeFeatures {
    #[cfg(not(feature = "wasmtime"))]
    fn default() -> Self {
        WasmRuntimeFeatures {
            name: "wasmi".to_string(),
            version: "0.31.0".to_string(),
            is_browser: false,
            supported_features: WasmFeatures::supported(),
            default_features: WasmFeatures::default(),
        }
    }

    #[cfg(feature = "wasmtime")]
    fn default() -> Self {
        WasmRuntimeFeatures {
            name: "wasmtime".to_string(),
            version: "14.0.4".to_string(),
            is_browser: false,
            supported_features: WasmFeatures::supported(),
            default_features: WasmFeatures::default(),
        }
    }
}

#[derive(Debug)]
pub struct ModuleConfig {
    /// Is `true` if the [`multi-value`] Wasm proposal is enabled.
    pub multi_value: Option<bool>,
    /// Is `true` if the [`bulk-memory`] Wasm proposal is enabled.
    pub bulk_memory: Option<bool>,
    /// Is `true` if the [`reference-types`] Wasm proposal is enabled.
    pub reference_types: Option<bool>,
    /// Is `true` if executions shall consume fuel.
    pub consume_fuel: Option<bool>,
    /// Configuration specific to the wasmi runtime
    pub wasmi: Option<ModuleConfigWasmi>,
    /// Configuration specific to the wasmtime runtime
    pub wasmtime: Option<ModuleConfigWasmtime>,
}

#[cfg(feature = "wasmtime")]
impl From<ModuleConfig> for wasmtime::Config {
    fn from(c: ModuleConfig) -> Self {
        let mut config = Self::new();
        c.multi_value.map(|v| config.wasm_multi_value(v));
        c.bulk_memory.map(|v| config.wasm_bulk_memory(v));
        c.reference_types.map(|v| config.wasm_reference_types(v));
        c.consume_fuel.map(|v| config.consume_fuel(v));
        if let Some(wtc) = c.wasmtime {
            // TODO: feature incremental-cache
            // wtc.enable_incremental_compilation.map(|v| config.enable_incremental_compilation(v));
            // wtc.async_support.map(|v| config.async_support(v));
            wtc.debug_info.map(|v| config.debug_info(v));
            wtc.wasm_backtrace.map(|v| config.wasm_backtrace(v));
            wtc.native_unwind_info.map(|v| config.native_unwind_info(v));
            // wtc.epoch_interruption.map(|v| config.epoch_interruption(v));
            wtc.max_wasm_stack.map(|v| config.max_wasm_stack(v));
            wtc.wasm_simd.map(|v| config.wasm_simd(v));
            wtc.wasm_relaxed_simd.map(|v| config.wasm_relaxed_simd(v));
            wtc.relaxed_simd_deterministic
                .map(|v| config.relaxed_simd_deterministic(v));
            wtc.wasm_threads.map(|v| config.wasm_threads(v));
            wtc.wasm_multi_memory.map(|v| config.wasm_multi_memory(v));
            // TODO: wtc.tail_call.map(|v| config.wasm_tail_call(v));
            wtc.wasm_memory64.map(|v| config.wasm_memory64(v));
            // TODO: feature component-model
            // wtc.wasm_component_model.map(|v| config.wasm_component_model(v));
            wtc.static_memory_maximum_size
                .map(|v| config.static_memory_maximum_size(v));
            wtc.static_memory_forced
                .map(|v| config.static_memory_forced(v));
            wtc.static_memory_guard_size
                .map(|v| config.static_memory_guard_size(v));
            wtc.parallel_compilation
                .map(|v| config.parallel_compilation(v));
            wtc.generate_address_map
                .map(|v| config.generate_address_map(v));
        }
        config
    }
}

#[cfg(not(feature = "wasmtime"))]
impl From<ModuleConfig> for wasmi::Config {
    fn from(c: ModuleConfig) -> Self {
        let mut config = Self::default();
        c.multi_value.map(|v| config.wasm_multi_value(v));
        c.bulk_memory.map(|v| config.wasm_bulk_memory(v));
        c.reference_types.map(|v| config.wasm_reference_types(v));
        c.consume_fuel.map(|v| config.consume_fuel(v));
        if let Some(wic) = c.wasmi {
            wic.stack_limits
                .map(|v| config.set_stack_limits(v.try_into().unwrap()));
            wic.cached_stacks.map(|v| config.set_cached_stacks(v));
            wic.mutable_global.map(|v| config.wasm_mutable_global(v));
            wic.sign_extension.map(|v| config.wasm_sign_extension(v));
            wic.saturating_float_to_int
                .map(|v| config.wasm_saturating_float_to_int(v));
            wic.tail_call.map(|v| config.wasm_tail_call(v));
            wic.extended_const.map(|v| config.wasm_extended_const(v));
            wic.floats.map(|v| config.floats(v));
            // config.set_fuel_costs(wic.flue_costs);
        }
        config
    }
}

#[derive(Debug)]
pub struct ModuleConfigWasmi {
    /// The limits set on the value stack and call stack.
    pub stack_limits: Option<WasiStackLimits>,
    /// The amount of Wasm stacks to keep in cache at most.
    pub cached_stacks: Option<usize>,
    /// Is `true` if the `mutable-global` Wasm proposal is enabled.
    pub mutable_global: Option<bool>,
    /// Is `true` if the `sign-extension` Wasm proposal is enabled.
    pub sign_extension: Option<bool>,
    /// Is `true` if the `saturating-float-to-int` Wasm proposal is enabled.
    pub saturating_float_to_int: Option<bool>,
    /// Is `true` if the [`tail-call`] Wasm proposal is enabled.
    pub tail_call: Option<bool>, // wasmtime disabled
    /// Is `true` if the [`extended-const`] Wasm proposal is enabled.
    pub extended_const: Option<bool>,
    /// Is `true` if Wasm instructions on `f32` and `f64` types are allowed.
    pub floats: Option<bool>,
    // /// The fuel consumption mode of the `wasmi` [`Engine`](crate::Engine).
    // // TODO: pub fuel_consumption_mode: FuelConsumptionMode,
    // /// The configured fuel costs of all `wasmi` bytecode instructions.
    // // pub fuel_costs: FuelCosts,
}

/// The configured limits of the Wasm stack.
#[derive(Debug, Copy, Clone)]
pub struct WasiStackLimits {
    /// The initial value stack height that the Wasm stack prepares.
    pub initial_value_stack_height: usize,
    /// The maximum value stack height in use that the Wasm stack allows.
    pub maximum_value_stack_height: usize,
    /// The maximum number of nested calls that the Wasm stack allows.
    pub maximum_recursion_depth: usize,
}

#[cfg(not(feature = "wasmtime"))]
impl TryFrom<WasiStackLimits> for wasmi::StackLimits {
    type Error = anyhow::Error;

    fn try_from(value: WasiStackLimits) -> std::result::Result<Self, Self::Error> {
        use crate::types::to_anyhow;

        Self::new(
            value.initial_value_stack_height,
            value.maximum_value_stack_height,
            value.maximum_recursion_depth,
        )
        .map_err(to_anyhow)
    }
}

#[derive(Debug)]
pub struct ModuleConfigWasmtime {
    // TODO: pub enable_incremental_compilation: Option<bool>, incremental-cache feature
    // TODO: pub async_support: Option<bool>,                  async feature
    /// Configures whether DWARF debug information will be emitted during
    /// compilation.
    pub debug_info: Option<bool>,
    pub wasm_backtrace: Option<bool>,
    pub native_unwind_info: Option<bool>,
    // TODO: pub wasm_backtrace_details: WasmBacktraceDetails, // Or WASMTIME_BACKTRACE_DETAILS env var
    //
    // TODO: pub epoch_interruption: Option<bool>, // vs consume_fuel
    pub max_wasm_stack: Option<usize>,
    /// Whether or not to enable the `threads` WebAssembly feature.
    /// This includes atomics and shared memory as well.
    /// This is not enabled by default.
    pub wasm_threads: Option<bool>,
    /// Whether or not to enable the `simd` WebAssembly feature.
    pub wasm_simd: Option<bool>,
    /// Whether or not to enable the `relaxed-simd` WebAssembly feature.
    /// This is not enabled by default.
    pub wasm_relaxed_simd: Option<bool>,
    /// Whether [wasm_relaxed_simd] should be deterministic.
    /// This is false by default.
    pub relaxed_simd_deterministic: Option<bool>,
    /// Whether or not to enable the `multi-memory` WebAssembly feature.
    /// This is not enabled by default.
    pub wasm_multi_memory: Option<bool>,
    /// Whether or not to enable the `memory64` WebAssembly feature.
    /// This is not enabled by default.
    pub wasm_memory64: Option<bool>,
    // TODO: pub wasm_component_model: Option<bool>, // false component-model feature
    //
    // pub strategy: Strategy,
    // TODO: pub profiler: ProfilingStrategy,
    // TODO: pub allocation_strategy: OnDemand, // vs Polling feature flag
    pub static_memory_maximum_size: Option<u64>,
    pub static_memory_forced: Option<bool>,
    pub static_memory_guard_size: Option<u64>,
    pub parallel_compilation: Option<bool>,
    pub generate_address_map: Option<bool>,
}

/// https://docs.wasmtime.dev/stability-wasm-proposals-support.html
pub struct WasmFeatures {
    /// The WebAssembly `mutable-global` proposal (enabled by default)
    pub mutable_global: bool,
    /// The WebAssembly `nontrapping-float-to-int-conversions` proposal (enabled by default)
    pub saturating_float_to_int: bool,
    /// The WebAssembly `sign-extension-ops` proposal (enabled by default)
    pub sign_extension: bool,
    /// The WebAssembly reference types proposal (enabled by default)
    pub reference_types: bool,
    /// The WebAssembly multi-value proposal (enabled by default)
    pub multi_value: bool,
    /// The WebAssembly bulk memory operations proposal (enabled by default)
    pub bulk_memory: bool,
    /// The WebAssembly SIMD proposal
    pub simd: bool,
    /// The WebAssembly Relaxed SIMD proposal
    pub relaxed_simd: bool,
    /// The WebAssembly threads proposal, shared memory and atomics
    /// https://docs.rs/wasmtime/14.0.4/wasmtime/struct.Config.html#method.wasm_threads
    pub threads: bool,
    /// The WebAssembly tail-call proposal
    pub tail_call: bool,
    /// Whether or not floating-point instructions are enabled.
    ///
    /// This is enabled by default can be used to disallow floating-point
    /// operators and types.
    ///
    /// This does not correspond to a WebAssembly proposal but is instead
    /// intended for embeddings which have stricter-than-usual requirements
    /// about execution. Floats in WebAssembly can have different NaN patterns
    /// across hosts which can lead to host-dependent execution which some
    /// runtimes may not desire.
    pub floats: bool,
    /// The WebAssembly multi memory proposal
    pub multi_memory: bool,
    /// The WebAssembly exception handling proposal
    pub exceptions: bool,
    /// The WebAssembly memory64 proposal
    pub memory64: bool,
    /// The WebAssembly extended_const proposal
    pub extended_const: bool,
    /// The WebAssembly component model proposal
    pub component_model: bool,
    /// The WebAssembly memory control proposal
    pub memory_control: bool,
    /// The WebAssembly garbage collection (GC) proposal
    pub garbage_collection: bool,
    /// WebAssembly external types reflection or, for browsers,
    /// the js-types proposal (https://github.com/WebAssembly/js-types/blob/main/proposals/js-types/Overview.md)
    pub type_reflection: bool,
    /// The WebAssembly System Interface proposal
    pub wasi_features: Option<WasmWasiFeatures>,
    // TODO:
    //   final bool moduleLinking;
}

/// https://docs.wasmtime.dev/stability-wasi-proposals-support.html
pub struct WasmWasiFeatures {
    // TODO: pub snapshot_preview1: bool,
    /// Access to standard input, output, and error streams
    pub io: bool,
    /// Access to the filesystem
    pub filesystem: bool,
    /// Access to clocks and the system time
    pub clocks: bool,
    /// Access to random number generators
    pub random: bool,
    pub poll: bool,
    /// wasi-nn
    pub machine_learning: bool,
    /// wasi-crypto
    pub crypto: bool,
    /// WASM threads with ability to spawn
    /// https://github.com/WebAssembly/wasi-threads
    pub threads: bool,
}

impl WasmWasiFeatures {
    /// Returns the default set of Wasi features.
    pub fn default() -> WasmWasiFeatures {
        WasmWasiFeatures {
            io: true,
            filesystem: true,
            clocks: true,
            random: true,
            poll: true,
            // TODO: implement through separate libraries
            machine_learning: false,
            crypto: false,
            // Unsupported
            threads: false,
        }
    }

    pub fn supported() -> WasmWasiFeatures {
        WasmWasiFeatures::default()
    }
}

impl WasmFeatures {
    /// Returns the default set of Wasm features.
    pub fn default() -> WasmFeatures {
        #[cfg(feature = "wasmtime")]
        {
            return WasmFeatures {
                multi_value: true,
                bulk_memory: true,
                reference_types: true,
                mutable_global: true,
                saturating_float_to_int: true,
                sign_extension: true,
                extended_const: true,
                floats: true,
                simd: true,
                relaxed_simd: false,
                threads: false,      // Default false
                multi_memory: false, // Default false
                memory64: false,     // Default false
                // Unsupported
                component_model: false, // Feature
                garbage_collection: false,
                tail_call: false,
                exceptions: false,
                memory_control: false,
                type_reflection: true,
                wasi_features: if cfg!(feature = "wasi") {
                    Some(WasmWasiFeatures::default())
                } else {
                    None
                },
            };
        }
        // TODO: use features crate
        #[allow(unreachable_code)]
        WasmFeatures {
            multi_value: true,
            bulk_memory: true,
            reference_types: true,
            mutable_global: true,
            saturating_float_to_int: true,
            sign_extension: true,
            tail_call: false,      // Default false
            extended_const: false, // Default false
            floats: true,
            // Unsupported
            component_model: false,
            garbage_collection: false,
            simd: false,
            relaxed_simd: false,
            threads: false,
            multi_memory: false,
            exceptions: false,
            memory64: false,
            memory_control: false,
            type_reflection: true,
            wasi_features: if cfg!(feature = "wasi") {
                Some(WasmWasiFeatures::default())
            } else {
                None
            },
        }
    }

    pub fn supported() -> WasmFeatures {
        #[cfg(feature = "wasmtime")]
        {
            return WasmFeatures {
                multi_value: true,
                bulk_memory: true,
                reference_types: true,
                mutable_global: true,
                saturating_float_to_int: true,
                sign_extension: true,
                extended_const: true,
                floats: true,
                simd: true,
                relaxed_simd: true,
                threads: true,
                multi_memory: true,
                memory64: true,
                // Unsupported
                component_model: false, // Feature
                garbage_collection: false,
                exceptions: false,
                tail_call: false,
                memory_control: false,
                type_reflection: true,
                wasi_features: if cfg!(feature = "wasi") {
                    Some(WasmWasiFeatures::supported())
                } else {
                    None
                },
            };
        }
        // TODO: use features crate
        #[allow(unreachable_code)]
        WasmFeatures {
            multi_value: true,
            bulk_memory: true,
            reference_types: true,
            mutable_global: true,
            saturating_float_to_int: true,
            sign_extension: true,
            tail_call: true,
            extended_const: true,
            floats: true,
            // Unsupported
            component_model: false,
            garbage_collection: false,
            simd: false,
            relaxed_simd: false,
            threads: false,
            multi_memory: false,
            exceptions: false,
            memory64: false,
            memory_control: false,
            type_reflection: true,
            wasi_features: if cfg!(feature = "wasi") {
                Some(WasmWasiFeatures::supported())
            } else {
                None
            },
        }
    }
}

impl ModuleConfig {
    /// Returns the [`WasmFeatures`] represented by the [`ModuleConfig`].
    // TODO: use features crate
    #[allow(unreachable_code)]
    pub fn wasm_features(&self) -> WasmFeatures {
        #[cfg(feature = "wasmtime")]
        {
            let w = self.wasmtime.as_ref();
            let def = WasmFeatures::default();
            return WasmFeatures {
                multi_value: self.multi_value.unwrap_or(def.multi_value),
                bulk_memory: self.bulk_memory.unwrap_or(def.bulk_memory),
                reference_types: self.reference_types.unwrap_or(def.reference_types),
                // True by default, can't be configured
                mutable_global: true,
                saturating_float_to_int: true,
                sign_extension: true,
                extended_const: true,
                floats: true,

                simd: w.and_then(|w| w.wasm_simd).unwrap_or(def.simd),
                threads: w.and_then(|w| w.wasm_threads).unwrap_or(def.threads),
                multi_memory: w
                    .and_then(|w| w.wasm_multi_memory)
                    .unwrap_or(def.multi_memory),
                memory64: w.and_then(|w| w.wasm_memory64).unwrap_or(def.memory64),
                relaxed_simd: w
                    .and_then(|w| w.wasm_relaxed_simd)
                    .unwrap_or(def.relaxed_simd),
                // Unsupported
                component_model: false, // Feature
                garbage_collection: false,
                tail_call: false,
                exceptions: false,
                memory_control: false,
                type_reflection: true,
                wasi_features: if cfg!(feature = "wasi") {
                    Some(WasmWasiFeatures::default())
                } else {
                    None
                },
            };
        }
        let w = self.wasmi.as_ref();
        let def = WasmFeatures::default();
        WasmFeatures {
            multi_value: self.multi_value.unwrap_or(def.multi_value),
            bulk_memory: self.bulk_memory.unwrap_or(def.bulk_memory),
            reference_types: self.reference_types.unwrap_or(def.reference_types),
            mutable_global: w
                .and_then(|w| w.mutable_global)
                .unwrap_or(def.mutable_global),
            saturating_float_to_int: w
                .and_then(|w| w.saturating_float_to_int)
                .unwrap_or(def.saturating_float_to_int),
            sign_extension: w
                .and_then(|w| w.sign_extension)
                .unwrap_or(def.sign_extension),
            tail_call: w.and_then(|w| w.tail_call).unwrap_or(def.tail_call),
            extended_const: w
                .and_then(|w| w.extended_const)
                .unwrap_or(def.extended_const),
            floats: w.and_then(|w| w.floats).unwrap_or(def.floats),
            // Unsupported
            garbage_collection: false,
            component_model: false,
            simd: false,
            relaxed_simd: false,
            threads: false,
            multi_memory: false,
            exceptions: false,
            memory64: false,
            memory_control: false,
            type_reflection: true,
            wasi_features: if cfg!(feature = "wasi") {
                Some(WasmWasiFeatures::default())
            } else {
                None
            },
        }
    }
}
