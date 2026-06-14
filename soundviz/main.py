import numpy as np
import matplotlib.pyplot as plt
import sounddevice as sd
import warnings
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
        "name": "Reel Equation",
        "desc": "5 * Σ (1/(k+1)) * sin((4k+1)x), k=0..12",
        "fn": lambda t: 5 * sum((1/(k+1)) * np.sin((4*k+1)*t) for k in range(13))
    },
    "5": {
        "name": "Pulse Wave",
        "desc": "Σ sin(n*x)/n, n=1,3,5..19",
        "fn": lambda t: sum(np.sin(n*t)/n for n in range(1, 20, 2))
    },
}


def normalize(wave):
    peak = np.max(np.abs(wave))
    return wave / peak * 0.8 if peak > 0 else wave


def compute_fft(wave):
    n = len(wave)
    freq = np.fft.fftfreq(n, 1/SAMPLE_RATE)
    mag = np.abs(np.fft.fft(wave))
    mask = (freq >= 0) & (freq <= 3000)
    return freq[mask], mag[mask]


def hear_as_timbre(fn):
    """wave shape used directly as audio — hear the harmonic content"""
    t = np.linspace(0, 2 * np.pi * 440 * DURATION,
                    int(SAMPLE_RATE * DURATION), endpoint=False)
    wave = fn(t / SAMPLE_RATE)
    return normalize(wave)


def hear_as_melody(fn):
    """
    wave Y value → pitch (FM style, like the reel video)
    wave goes up = pitch goes up, wave goes down = pitch goes down
    """
    # sample the wave shape at low resolution (this is the "score")
    t_shape = np.linspace(
        0, 4 * np.pi, int(SAMPLE_RATE * DURATION), endpoint=False)
    shape = fn(t_shape)

    # normalize shape to frequency range (200Hz to 1200Hz)
    shape_n = (shape - shape.min()) / (shape.max() - shape.min())
    freqs = 200 + shape_n * 1000  # maps 0..1 to 200..1200 Hz

    # generate audio by integrating instantaneous frequency
    phase = np.cumsum(2 * np.pi * freqs / SAMPLE_RATE)
    wave = np.sin(phase)
    return normalize(wave)


def plot_wave(name, fn, mode):
    t_plot = np.linspace(0, 4 * np.pi, 1000)
    wave_plot = fn(t_plot)

    if mode == '1':
        audio = hear_as_timbre(fn)
        mode_label = "Timbre mode"
    else:
        audio = hear_as_melody(fn)
        mode_label = "Melody mode (FM)"

    freq, mag = compute_fft(audio)
    sample_20ms = int(SAMPLE_RATE * 0.02)

    # --- static plots saved as PNG ---
    fig, axes = plt.subplots(3, 1, figsize=(12, 9))
    fig.patch.set_facecolor('#0d0d0d')
    for ax in axes:
        ax.set_facecolor('#0d0d0d')
        ax.tick_params(colors='white')
        ax.xaxis.label.set_color('white')
        ax.yaxis.label.set_color('white')
        ax.title.set_color('white')
        for spine in ax.spines.values():
            spine.set_edgecolor('#333')

    fig.suptitle(f"{name} — {mode_label}", fontsize=13,
                 color='white', fontweight='bold')

    axes[0].plot(t_plot, wave_plot, color='cyan', linewidth=1.5)
    axes[0].set_title('Wave Shape')
    axes[0].set_xlabel('x')
    axes[0].set_ylabel('Amplitude')
    axes[0].grid(True, alpha=0.2)
    axes[0].axhline(0, color='white', linewidth=0.5, alpha=0.4)

    axes[1].plot(np.linspace(0, 20, sample_20ms),
                 audio[:sample_20ms], color='lime', linewidth=1)
    axes[1].set_title('Audio Waveform (first 20ms)')
    axes[1].set_xlabel('Time (ms)')
    axes[1].set_ylabel('Amplitude')
    axes[1].grid(True, alpha=0.2)

    axes[2].plot(freq, mag, color='magenta', linewidth=1)
    axes[2].fill_between(freq, mag, alpha=0.25, color='magenta')
    axes[2].set_title('FFT Spectrum')
    axes[2].set_xlabel('Frequency (Hz)')
    axes[2].set_ylabel('Magnitude')
    axes[2].grid(True, alpha=0.2)

    plt.tight_layout()
    safe_name = name.replace(' ', '_').replace(':', '').replace('*', 'x')[:40]
    png_path = f"{safe_name}_{mode_label.split()[0]}.png"
    plt.savefig(png_path, dpi=150, facecolor='#0d0d0d')
    plt.close()
    print(f"Plot saved: {png_path}")

    # --- animated GIF of wave drawing itself ---
    from matplotlib.animation import FuncAnimation, PillowWriter

    fig2, ax = plt.subplots(figsize=(10, 4))
    fig2.patch.set_facecolor('#0d0d0d')
    ax.set_facecolor('#0d0d0d')
    ax.tick_params(colors='white')
    ax.xaxis.label.set_color('white')
    ax.yaxis.label.set_color('white')
    ax.title.set_color('white')
    for spine in ax.spines.values():
        spine.set_edgecolor('#333')

    ax.set_xlim(t_plot[0], t_plot[-1])
    ax.set_ylim(wave_plot.min() * 1.2, wave_plot.max() * 1.2)
    ax.set_title(f"{name}", color='white')
    ax.axhline(0, color='white', linewidth=0.5, alpha=0.4)
    ax.grid(True, alpha=0.2)

    line, = ax.plot([], [], color='cyan', linewidth=2)
    dot,  = ax.plot([], [], 'o', color='yellow', markersize=6)

    frames = 80

    def animate(i):
        end = int((i + 1) * len(t_plot) / frames)
        line.set_data(t_plot[:end], wave_plot[:end])
        if end > 0:
            dot.set_data([t_plot[end-1]], [wave_plot[end-1]])
        return line, dot

    ani = FuncAnimation(fig2, animate, frames=frames, interval=30, blit=True)
    gif_path = f"{safe_name}.gif"
    ani.save(gif_path, writer=PillowWriter(fps=30))
    plt.close()
    print(f"GIF saved:  {gif_path}")

    # --- play audio ---
    print(f"Playing ({mode_label})...")
    sd.play(audio, SAMPLE_RATE)
    sd.wait()
    print("Done.")


def parse_custom(expr_str):
    """parse simple expressions using x as variable, k as index"""
    import sympy as sp
    x = sp.Symbol('x')
    try:
        expr = sp.sympify(expr_str)
        fn = sp.lambdify(x, expr, modules=['numpy'])
        fn(np.array([0.0, 1.0]))  # test
        return fn
    except Exception as e:
        print(f"Parse error: {e}")
        return None


def main():
    print("=" * 50)
    print("  Equation → Sound")
    print("=" * 50)

    while True:
        print("\nPresets:")
        for k, v in PRESETS.items():
            print(f"  {k}. {v['name']:<20} {v['desc']}")
        print("  c. Custom equation")
        print("  q. Quit")

        choice = input("\nChoose: ").strip().lower()
        if choice == 'q':
            break

        if choice == 'c':
            print("\nExamples:")
            print("  sin(x) + sin(3*x)/3 + sin(5*x)/5")
            print("  sin(x) * cos(2*x)")
            print("  sin(x) + 0.5*sin(2*x) + 0.25*sin(4*x)")
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
            print("Invalid choice.")
            continue

        print("\nHow to hear it:")
        print("  1. Timbre mode  — wave shape as direct audio (hear harmonics)")
        print(
            "  2. Melody mode  — wave shape controls pitch (wobbling sound like the reel)")
        mode = input("Choose (1/2): ").strip()
        if mode not in ('1', '2'):
            mode = '1'

        plot_wave(name, fn, mode)

        input("\nPress Enter to continue...")
        plt.close('all')


if __name__ == '__main__':
    main()
