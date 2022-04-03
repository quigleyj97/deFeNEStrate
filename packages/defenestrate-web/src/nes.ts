import React from "react";
import { NesEmulator, init_debug_hooks } from "../../defenestrate-core/pkg"
import { convertEmuBufferToImageData } from "./utils/buffer";

declare global {
    namespace JSX {
        interface IntrinsicElements {
            'nes-emulator': React.DetailedHTMLProps<React.HTMLAttributes<HTMLNesEmulatorElement>, HTMLNesEmulatorElement>
        }
    }
}

interface IWasmModule {
    NesEmulator: typeof NesEmulator;
    init_debug_hooks: typeof init_debug_hooks;
}

// The values are ordered, except for Error. This means you can test if loading
// has progressed to a specific state or beyond with a comparison, eg. 
// `loading >= WASM_LOADED` means the WASM binaries are ready to execute.
export enum LoadingState {
    // No loading started.
    UNINITIALIZED,
    // Fetching WASM binaries over the wire
    LOADING_WASM,
    // WASM fetch completed and binary transpiled, debug hooks initialized.
    WASM_LOADED,
    // Emulator initialized successfully, ready to start emulation
    READY,
    // Something failed to load, and we couldn't recover from it.
    ERROR = -1
}

export class HTMLNesEmulatorElement extends HTMLElement {
    private module?: IWasmModule;
    private loading = LoadingState.UNINITIALIZED;
    private emulator?: NesEmulator;
    private canvas: HTMLCanvasElement | null = null;
    private renderingContext?: CanvasRenderingContext2D;

    constructor() {
        super();
        this.attachShadow({ mode: "open" });
    }

    public async init() {
        this.loading = LoadingState.LOADING_WASM;
        const { init_debug_hooks, NesEmulator } = await import("../../defenestrate-core/pkg");
        this.module = { init_debug_hooks, NesEmulator };
        try {
            init_debug_hooks();
        } catch (error) {
            // we _could_ continue from this but I honestly don't expect this call
            // to ever fail, so a failure here likely means something is wrong
            // with the WASM binary.
            console.error("Failed to init debug hooks, original error:");
            console.error(error);
            this.loading = LoadingState.ERROR;
            throw new Error("Failed to init debug hooks");
        }
        this.loading = LoadingState.WASM_LOADED;
    }

    public loadRom(rom: ArrayBuffer) {
        if (!this.isModuleReady(this.module)) {
            throw Error("Bad state: WASM not loaded")
        }
        try {
            this.emulator = new this.module.NesEmulator(new Uint8Array(rom));
        } catch (error) {
            console.error("Unexpected error when attempting to instantiate emulator:");
            console.error(error);
            this.loading = LoadingState.ERROR;
            throw new Error("Failed to instantiate emulator");
        }
        this.loading = LoadingState.READY;
    }

    public run_frame() {
        if (!this.isEmulatorReady(this.emulator)) {
            throw Error("Bad state: Emulator not loaded")
        }
        const output = this.emulator.step_frame();
        const frame = convertEmuBufferToImageData(output, 256, 240);
        this.renderingContext!.putImageData(frame, 0, 0);
    }


    /** WebComponent hook, not part of public API */
    connectedCallback() {
        this.render();
    }

    disconnectedCallback() {
        if (this.emulator) {
            this.emulator.free();
            this.emulator = void 0;
        }
        this.loading = LoadingState.WASM_LOADED;
    }

    private render() {
        // pixelated is what we want but not every browser supports it, so
        // we have crisp-edges as a fallback
        this.innerHTML = `
            <style>
                canvas#emu-screen {
                    image-rendering: crisp-edges;
                    image-rendering: pixelated;
                }
            </style>
            <canvas id="emu-screen" width="256" height="240"></canvas>
        `;
        this.canvas = this.querySelector("#emu-screen");
        this.renderingContext = this.canvas!.getContext("2d", {
            // nes has no support for alpha
            alpha: false
        })!;
    }

    private isModuleReady(mod?: IWasmModule): mod is IWasmModule {
        return this.loading >= LoadingState.WASM_LOADED;
    }

    private isEmulatorReady(emu?: NesEmulator): emu is NesEmulator {
        return this.loading >= LoadingState.READY;
    }
}

window.customElements.define('nes-emulator', HTMLNesEmulatorElement);