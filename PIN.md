# Pinouts for our project

```ascii
+--------------+-----+-----+---------+------+---+Pi Zero 2W+---+------+---------+-----+-----+--------------+
|     Function | BCM | wPi |   Name  | Mode | V | Physical | V | Mode | Name    | wPi | BCM |     Function |
+--------------+-----+-----+---------+------+---+----++----+---+------+---------+-----+-----+--------------+
|   WM8960 3V3 |     |     |    3.3v |      |   |  1 || 2  |   |      | 5v      |     |     |   WM8960  5V |
|   WM8960 SDA |   2 |   8 |   SDA.1 | ALT0 | 1 |  3 || 4  |   |      | 5v      |     |     |       LED 5V |
|   WM8960 SCL |   3 |   9 |   SCL.1 | ALT0 | 1 |  5 || 6  |   |      | 0v      |     |     |          GND |
|    Button 01 |   4 |   7 | GPIO. 7 |   IN | 1 |  7 || 8  | 1 | ALT5 | TxD     | 15  | 14  |            - |
|          GND |     |     |      0v |      |   |  9 || 10 | 1 | ALT5 | RxD     | 16  | 15  |            - |
|    Button 02 |  17 |   0 | GPIO. 0 |   IN | 0 | 11 || 12 | 0 | ALT0 | GPIO. 1 | 1   | 18  |  WM8960 BCLK |
|    Button 03 |  27 |   2 | GPIO. 2 |   IN | 0 | 13 || 14 |   |      | 0v      |     |     |          GND |
|    Button 04 |  22 |   3 | GPIO. 3 |   IN | 0 | 15 || 16 | 0 | IN   | GPIO. 4 | 4   | 23  |    Button 09 |
|         3.3V |     |     |    3.3v |      |   | 17 || 18 | 0 | IN   | GPIO. 5 | 5   | 24  |    Button 10 |
|     LED DATA |  10 |  12 |    MOSI |   IN | 0 | 19 || 20 |   |      | 0v      |     |     |          GND |
|           -  |   9 |  13 |    MISO |   IN | 0 | 21 || 22 | 0 | IN   | GPIO. 6 | 6   | 25  |    Button 11 |
|      LED CLK |  11 |  14 |    SCLK |   IN | 0 | 23 || 24 | 1 | IN   | CE0     | 10  | 8   |    LED LATCH |
|          GND |     |     |      0v |      |   | 25 || 26 | 1 | IN   | CE1     | 11  | 7   |            - |
|           -  |   0 |  30 |   SDA.0 |   IN | 1 | 27 || 28 | 1 | IN   | SCL.0   | 31  | 1   |            - |
|    Button 05 |   5 |  21 | GPIO.21 |   IN | 1 | 29 || 30 |   |      | 0v      |     |     |          GND |
|    Button 06 |   6 |  22 | GPIO.22 |   IN | 1 | 31 || 32 | 0 | IN   | GPIO.26 | 26  | 12  |    Button 12 |
|    Button 07 |  13 |  23 | GPIO.23 |   IN | 0 | 33 || 34 |   |      | 0v      |     |     |          GND |
|    Button 08 |  19 |  24 | GPIO.24 | ALT0 | 0 | 35 || 36 | 0 | IN   | GPIO.27 | 27  | 16  |    Button 13 |
| WM8960 LRCLK |  26 |  25 | GPIO.25 |   IN | 0 | 37 || 38 | 0 | ALT0 | GPIO.28 | 28  | 20  |  WM8960 ADAT |
|   HP OUT GND |     |     |      0v |      |   | 39 || 40 | 0 | ALT0 | GPIO.29 | 29  | 21  |  WM8960 DDAT |
+--------------+-----+-----+---------+------+---+----++----+---+------+---------+-----+-----+--------------+
|     Function | BCM | wPi |   Name  | Mode | V | Physical | V | Mode | Name    | wPi | BCM |     Function |
+--------------+-----+-----+---------+------+---+Pi Zero 2W+---+------+---------+-----+-----+--------------+
```