# Plans on implementing LOOOPER `v0.1.0`

## Planned user experience (example workflow)

- User connects to the Pi via SSH or other means
- User setup the BPM, time signature, routing...
- <Space> User start the preparing phase
  - Recorded audio should be routed into headphone (monitoring)
    - User should be able to adjust the volume now
  - The user is also able to setup which loop the audio get recorded into immediately after the count-in
- <Space> User feels comfortable starting the performance and start the Count-in
  - The length of the count-in can be adjusted before the preparing phase
- The first loop starts and user starts performing.
  - The audio gets routed to the second loop and the monitoring headphone.
  - The length of the second loop is set to a single measure, and recording mode is set to once, thus the loop stops receiving new audio after a measure and start repeating its content into the mix.
- In measure two, user setup loop 2 to record in (and only in) the next measure.
- In measure three, loop 2 continue playing its previous content while simultaneously recording the newly captured audio.
- In measure four, loop 2 stops recording but plays the combination of the audio recorded in the first and the third measure.
