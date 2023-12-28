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
    - Simple repeat
    - Repeat with variation
    - Da Capo (D.C.)
    - Dal Segno (D.S.)
    - Fine
    - Coda
- Project
    - Key
    - Grid (Can snap to the horizontal position)
    - Rhythm
- Undo
