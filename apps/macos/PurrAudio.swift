import AVFoundation
import Foundation

/// Procedural cat purr audio driver backed by `keycat-core`'s DSP synth via `keycat-ffi`.
/// Starts `AVAudioEngine` lazily only when sound is enabled and the cat is actively purring,
/// and pauses the engine shortly after purring fades to silence so CoreAudio stays idle.
final class PurrAudio {
    private var audioPtr: OpaquePointer?
    private var engine: AVAudioEngine?
    private var sourceNode: AVAudioSourceNode?
    private var isRunning = false
    private var silentTicks = 0

    init() {
        self.audioPtr = kc_audio_new(48_000)
    }

    deinit {
        stop()
    }

    func sync(scene: OpaquePointer) {
        guard let ptr = audioPtr else { return }
        let active = kc_audio_sync_from_scene(ptr, scene) != 0
        if active {
            silentTicks = 0
            if !isRunning {
                startEngine(ptr: ptr)
            }
        } else if isRunning {
            silentTicks += 1
            if silentTicks >= 90 {
                stopEngine()
            }
        }
    }

    private func startEngine(ptr: OpaquePointer) {
        if engine == nil {
            guard let format = AVAudioFormat(standardFormatWithSampleRate: 48_000, channels: 1) else {
                return
            }
            let rawPtr = ptr
            let node = AVAudioSourceNode(format: format) { _, _, frameCount, audioBufferList -> OSStatus in
                let abl = UnsafeMutableAudioBufferListPointer(audioBufferList)
                if let first = abl.first, let data = first.mData {
                    let maxFrames = UInt32(first.mDataByteSize) / UInt32(MemoryLayout<Float>.size)
                    let count = min(maxFrames, frameCount)
                    let samples = data.assumingMemoryBound(to: Float.self)
                    kc_audio_fill_f32(rawPtr, samples, count)
                    let bytes = Int(count) * MemoryLayout<Float>.size
                    for i in 1..<abl.count {
                        if let dst = abl[i].mData {
                            memcpy(dst, data, bytes)
                        }
                    }
                }
                return noErr
            }
            let eng = AVAudioEngine()
            eng.attach(node)
            eng.connect(node, to: eng.mainMixerNode, format: format)
            self.engine = eng
            self.sourceNode = node
        }
        guard let eng = engine else { return }
        do {
            try eng.start()
            isRunning = true
        } catch {
            isRunning = false
        }
    }

    private func stopEngine() {
        if let eng = engine {
            eng.stop()
        }
        isRunning = false
        silentTicks = 0
    }

    func pause() {
        stopEngine()
    }

    func stop() {
        stopEngine()
        if let eng = engine, let node = sourceNode {
            eng.detach(node)
        }
        sourceNode = nil
        engine = nil
        if let ptr = audioPtr {
            kc_audio_free(ptr)
            audioPtr = nil
        }
    }
}
