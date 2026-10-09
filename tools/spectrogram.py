#!/usr/bin/env python3
"""Spectrogram + waveform of one or more WAV files, stacked into one PNG.

    python3 tools/spectrogram.py out.png a.wav b.wav ...
"""
import sys
import wave

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np


def load(path):
    with wave.open(path) as w:
        sr = w.getframerate()
        data = np.frombuffer(w.readframes(w.getnframes()), dtype=np.int16)
        data = data.reshape(-1, w.getnchannels()).astype(np.float32) / 32768.0
    return sr, data.mean(axis=1)


def main():
    out, paths = sys.argv[1], sys.argv[2:]
    fig, axes = plt.subplots(len(paths), 2, figsize=(16, 2.6 * len(paths)),
                             gridspec_kw={"width_ratios": [3, 1]}, squeeze=False)
    for (spec_ax, wave_ax), path in zip(axes, paths):
        sr, x = load(path)
        spec_ax.specgram(x + 1e-9, NFFT=2048, Fs=sr, noverlap=1792, cmap="magma",
                         vmin=-120, vmax=-20, scale="dB")
        spec_ax.set_yscale("symlog", linthresh=200)
        spec_ax.set_ylim(30, sr / 2)
        spec_ax.set_title(path.rsplit("/", 1)[-1], fontsize=9, loc="left")
        t = np.arange(len(x)) / sr
        wave_ax.plot(t, x, lw=0.3, color="k")
        wave_ax.set_ylim(-1, 1)
    fig.tight_layout()
    fig.savefig(out, dpi=70)


if __name__ == "__main__":
    main()
