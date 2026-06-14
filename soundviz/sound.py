import os
import re
import warnings
import threading
import soundfile as sf
import sounddevice as sd
import matplotlib.animation as animation
import matplotlib.pyplot as plt
import numpy as np
import matplotlib
matplotlib.use('Qt5Agg')
warnings.filterwarnings('ignore')

SAMPLE_RATE = 44100
DURATION = 3.0

PRESETS = {
    "1": {
        "name": "Square Wave",
        "desc": "Σ (1/(2k+1)) * sin((2k+1)x), k=0..20",
        "fn": lambda t: sum((1/(2*k+1)) * np.sin((2*k+1)*t) for k in range(21))
    },
    "2": {
        "name": "Sawtooth Wave",
        "desc": "Σ ((-1)^(k+1)/k) * sin(k*x), k=1..30",
        "fn": lambda t: sum(((-1)**(k+1)/k) * np.sin(k*t) for k in range(1, 31))
    },
    "3": {
        "name": "Triangle Wave",
        "desc": "(8/π²) Σ ((-1)^k/(2k+1)²) * sin((2k+1)x), k=0..20",
        "fn": lambda t: (8/np.pi**2) * sum(((-1)**k/(2*k+1)**2) * np.sin((2*k+1)*t) for k in range(21))
    },
    "4": {
        "name": "Custom Equation",
        "desc": "5 * Σ (1/(k+1)) * sin((4k+1)x), k=0..12",
        "fn": lambda t: 5 * sum((1/(k+1)) * np.sin((4*k+1)*t) for k in range(13))
    },
    "5": {
        "name": "Pulse Wave",
        "desc": "Σ sin(n*x)/n, n=1,3,5..19",
        "fn": lambda t: sum(np.sin(n*t)/n for n in range(1, 20, 2))
    },
}

# shared state between audio thread and animation


class AudioState:
    def __init__(self):
        self.current_frame = 0
        self.audio = None
        self.playing = False


state = AudioState()


def normalize(wave):
    peak = np.max(np.abs(wave))
    return wave / peak * 0.8 if peak > 0 else wave


def compute_fft(wave, max_freq=3000):
    n = len(wave)
    freq = np.fft.fftfreq(n, 1/SAMPLE_RATE)
    mag = np.abs(np.fft.fft(wave))
    mask = (freq >= 0) & (freq <= max_freq)
    return freq[mask], mag[mask]


def hear_as_timbre(fn):
    """direct harmonic audio — fixed by using proper time array"""
    n_samples = int(SAMPLE_RATE * DURATION)
    # generate one cycle of wave shape, then tile it at 440Hz
    samples_per_cycle = int(SAMPLE_RATE / 440)
    t_cycle = np.linspace(0, 2 * np.pi, samples_per_cycle, endpoint=False)
    one_cycle = fn(t_cycle)
    one_cycle = normalize(one_cycle)
    # tile to fill duration
    repeats = int(np.ceil(n_samples / samples_per_cycle))
    wave = np.tile(one_cycle, repeats)[:n_samples]
    return normalize(wave.astype(np.float32))


def hear_as_melody(fn):
    """wave Y value controls pitch — FM style"""
    t_shape = np.linspace(
        0, 4 * np.pi, int(SAMPLE_RATE * DURATION), endpoint=False)
    shape = fn(t_shape)
    shape_n = (shape - shape.min()) / (shape.max() - shape.min() + 1e-9)
    freqs = 200 + shape_n * 1000
    phase = np.cumsum(2 * np.pi * freqs / SAMPLE_RATE)
    wave = np.sin(phase)
    return normalize(wave.astype(np.float32))


def audio_thread_fn(audio):
    """play entire audio at once, track position via callback"""
    state.playing = True
    state.current_frame = 0

    def callback(outdata, frames, time, status):
        start = state.current_frame
        end = start + frames
        chunk = audio[start:end]
        if len(chunk) < frames:
            outdata[:len(chunk), 0] = chunk
            outdata[len(chunk):, 0] = 0
            state.current_frame = len(audio)
            state.playing = False
            raise sd.CallbackStop()
        outdata[:, 0] = chunk
        state.current_frame = end

    with sd.OutputStream(
        samplerate=SAMPLE_RATE,
        channels=1,
        callback=callback,
        dtype='float32'
    ) as stream:
        while state.playing:
            sd.sleep(50)


def live_visualizer(name, audio, mode_label):
    """animate waveform + FFT in sync with audio playback"""
    window = 2048   # samples shown in oscilloscope
    fft_window = 4096

    fig, axes = plt.subplots(2, 1, figsize=(12, 7))
    fig.patch.set_facecolor('#0d0d0d')
    for ax in axes:
        ax.set_facecolor('#0d0d0d')
        ax.tick_params(colors='white')
        ax.xaxis.label.set_color('white')
        ax.yaxis.label.set_color('white')
        ax.title.set_color('white')
        for spine in ax.spines.values():
            spine.set_edgecolor('#333')

    fig.suptitle(f"{name}  —  {mode_label}", fontsize=13,
                 color='white', fontweight='bold')

    # oscilloscope
    axes[0].set_xlim(0, window)
    axes[0].set_ylim(-1.1, 1.1)
    axes[0].set_title('Live Waveform')
    axes[0].set_xlabel('Samples')
    axes[0].set_ylabel('Amplitude')
    axes[0].grid(True, alpha=0.2)
    axes[0].axhline(0, color='white', linewidth=0.5, alpha=0.3)
    line_wave, = axes[0].plot(np.zeros(window), color='cyan', linewidth=1.5)

    # fft
    freq_full, _ = compute_fft(audio[:fft_window])
    axes[1].set_xlim(0, 3000)
    axes[1].set_ylim(0, np.max(np.abs(np.fft.fft(audio[:fft_window]))) * 0.6)
    axes[1].set_title('Live FFT Spectrum')
    axes[1].set_xlabel('Frequency (Hz)')
    axes[1].set_ylabel('Magnitude')
    axes[1].grid(True, alpha=0.2)
    line_fft,  = axes[1].plot(freq_full, np.zeros(
        len(freq_full)), color='magenta', linewidth=1)
    fill_fft = axes[1].fill_between(freq_full, np.zeros(
        len(freq_full)), alpha=0.25, color='magenta')

    plt.tight_layout()

    # start audio in background
    t = threading.Thread(target=audio_thread_fn, args=(audio,))
    t.start()

    def update(frame):
        cf = state.current_frame
        # oscilloscope slice
        start = max(0, cf - window // 2)
        end = start + window
        if end > len(audio):
            end = len(audio)
            start = max(0, end - window)
        slice_wave = audio[start:end]
        if len(slice_wave) < window:
            slice_wave = np.pad(slice_wave, (0, window - len(slice_wave)))
        line_wave.set_ydata(slice_wave)

        # fft slice
        fft_start = max(0, cf - fft_window // 2)
        fft_end = fft_start + fft_window
        if fft_end > len(audio):
            fft_end = len(audio)
            fft_start = max(0, fft_end - fft_window)
        slice_fft = audio[fft_start:fft_end]
        if len(slice_fft) < fft_window:
            slice_fft = np.pad(slice_fft, (0, fft_window - len(slice_fft)))
        _, mag = compute_fft(slice_fft)
        line_fft.set_ydata(mag)

        if not state.playing:
            ani.event_source.stop()

        return line_wave, line_fft

    ani = animation.FuncAnimation(
        fig, update, interval=30, blit=False
    )

    plt.show()
    state.playing = False
    t.join()


def parse_custom(expr_str):
    import sympy as sp
    x = sp.Symbol('x')
    expr_str = re.sub(r'(\d)([a-zA-Z(])', r'\1*\2', expr_str)
    try:
        expr = sp.sympify(expr_str)
        fn = sp.lambdify(x, expr, modules=['numpy'])
        fn(np.array([0.0, 1.0]))
        return fn
    except Exception as e:
        print(f"Parse error: {e}")
        return None


def mode_equation():
    while True:
        print("\nPresets:")
        for k, v in PRESETS.items():
            print(f"  {k}. {v['name']:<20} {v['desc']}")
        print("  c. Custom equation")
        print("  b. Back")

        choice = input("\nChoose: ").strip().lower()
        if choice == 'b':
            return

        if choice == 'c':
            print("\nExamples:")
            print("  sin(x) + sin(3*x)/3 + sin(5*x)/5")
            print("  sin(x) * cos(2*x)")
            expr_str = input("Enter equation (use x): ").strip()
            fn = parse_custom(expr_str)
            if not fn:
                continue
            name = f"Custom: {expr_str}"
        elif choice in PRESETS:
            p = PRESETS[choice]
            fn = p['fn']
            name = p['name']
        else:
            print("Invalid.")
            continue

        print("\n  1. Timbre mode  (hear harmonics directly)")
        print("  2. Melody mode  (wave shape controls pitch)")
        mode = input("Choose (1/2): ").strip()

        if mode == '1':
            audio = hear_as_timbre(fn)
            mode_label = "Timbre"
        else:
            audio = hear_as_melody(fn)
            mode_label = "Melody (FM)"

        live_visualizer(name, audio, mode_label)


def mode_audio_file():
    path = input("\nEnter path to audio file (.wav): ").strip()
    print(f"Checking path: '{path}'")
    print(f"Exists: {os.path.exists(path)}")
    if not os.path.exists(path):
        print("File not found.")
        return

    print(f"Loading {path}...")
    audio, sr = sf.read(path, dtype='float32')
    print(f"Loaded: {len(audio)} samples at {sr}Hz")

    # mix to mono if stereo
    if audio.ndim > 1:
        audio = audio.mean(axis=1)

    # resample if needed
    if sr != SAMPLE_RATE:
        print(f"Note: file is {sr}Hz, resampling to {SAMPLE_RATE}Hz")
        ratio = SAMPLE_RATE / sr
        audio = np.interp(
            np.arange(0, len(audio) * ratio),
            np.arange(0, len(audio)),
            audio
        ).astype(np.float32)

    audio = normalize(audio)
    print(f"Audio shape: {audio.shape}, max: {audio.max():.3f}")
    print("Launching visualizer...")

    live_visualizer(os.path.basename(path), audio, "Audio File")
    print("Visualizer returned")


def main():
    print("=" * 50)
    print("  Equation → Sound  |  Audio Visualizer")
    print("=" * 50)

    while True:
        print("\n  1. Equation → Sound")
        print("  2. Load audio file")
        print("  q. Quit")

        choice = input("\nChoose: ").strip().lower()
        if choice == 'q':
            break
        elif choice == '1':
            mode_equation()
        elif choice == '2':
            mode_audio_file()
        else:
            print("Invalid.")


if __name__ == '__main__':
    main()
