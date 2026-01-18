Hardware en-/decoding - GPU-Beschleunigung in der Videoverarbeitung

Encodinng -> NVIDIA GPU
          - name: IMMICH_FFMPEG_HWACCEL
            value: nvenc
Decoding -> cuda
          - name: IMMICH_FFMPEG_HWACCEL_DECODE
            value: cuda
Puffergröße für Datendurchsatz
          - name: IMMICH_FFMPEG_THREAD_QUEUE_SIZE
            value: "512"