# Playing order

- Repeat region is played twice. [See repeat examples](examples.md#repeat-region)
- Variation region is played as the following. [See variation examples](examples.md#variation-region)
    - Play common region then play the variation 1.
    - Play common region then play the variation 2.
    - ...
    - Play common region then play the variation N.
- Da Capo region is played as the following. [See Da Capo examples](examples.md#da-capo)
    - Play the entire regions as normal.
    - Play the regions from the top of the regions and play until it finds a Fine.
        - If it encounters a repeat region, play only once but twice.
        - If it encounters a variation region, play the common region and then only the last variation.
    - When the tune is auftakt, the following rule is applied.
        - Play the entire regions as normal.
        - If the last bar length is normal, that is equal to the full length calculated by the rhythm, go back to the second bar but the first one and play until it finds a Fine.
        - Otherwise play as normal D.C., go back to the top of the regions and play until it finds a Fine.
- Dal Segno region is played as the follwing.  [See Dal Segno examples](examples.md#dal-segno)
    - Play the entire regions as normal.
    - Go back to the Segno and play the regions until it finds a Fine.
        - If it encounters a repeat region, play only once but twice.
        - If it encounters a variation region, play the common region and then only the last variation.
    