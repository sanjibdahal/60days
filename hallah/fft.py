import numpy as np
import matplotlib.pyplot as plt

sample_rate = 44100  # samples per second
duration = 1.0       # seconds
t = np.linspace(0, duration, int(sample_rate * duration), endpoint=False)

# generate sine waves from scratch
f1, f2, f3 = 440, 880, 1320
sine1 = np.sin(2 * np.pi * f1 * t)
sine2 = 0.5 * np.sin(2 * np.pi * f2 * t)
sine3 = 0.25 * np.sin(2 * np.pi * f3 * t)

mixed = sine1 + sine2 + sine3

# compute FFT manually
fft_result = np.fft.fft(mixed)
frequencies = np.fft.fftfreq(len(mixed), 1 / sample_rate)
magnitude = np.abs(fft_result)

# plot
fig, axes = plt.subplots(3, 1, figsize=(12, 8))

axes[0].plot(t[:1000], mixed[:1000], color='cyan')
axes[0].set_title('Mixed Wave (time domain)')
axes[0].set_xlabel('Time (s)')
axes[0].set_ylabel('Amplitude')

axes[1].plot(t[:1000], sine1[:1000], label='440 Hz')
axes[1].plot(t[:1000], sine2[:1000], label='880 Hz')
axes[1].plot(t[:1000], sine3[:1000], label='1320 Hz')
axes[1].set_title('Individual Sine Waves')
axes[1].legend()

mask = (frequencies >= 0) & (frequencies <= 3000)
axes[2].plot(frequencies[mask], magnitude[mask], color='magenta')
axes[2].set_title('FFT — Frequency Domain')
axes[2].set_xlabel('Frequency (Hz)')
axes[2].set_ylabel('Magnitude')

plt.tight_layout()
plt.show()
