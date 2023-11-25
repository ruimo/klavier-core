# Abstract

Core library for MIDI sequencer. It does not depend on any GUI frameworks.

# Architecture

The following constructs are included.

- Bar
- Control change
- Note
    - Velocity
    - Duration
        - Tuple
    - Pitch
        - Solfa
        - Octave
        - Sharp flat
    - Trimmer (trim velocity, length, and start tick)
- Tempo
- Repeat
- Project
    - Key
    - Grid (Can snap to the horizontal position)
    - Rhythm
- Undo
