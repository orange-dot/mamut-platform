# Dobavljaci opreme za mamut-diverter (MCU arhitektura)

## Kontekst
Ovaj predlog je za sistem bez PLC-a: upravljanje zonama je na mikrokontrolerima (firmware), a integracija ide preko gateway/backend servisa. Fokus je na dobavljacima koji imaju industrijsku opremu pogodnu za integraciju preko Ethernet/DIO interfejsa.

## Dobavljaci po kategorijama
| Kategorija | Dobavljaci i primeri | Fit za MCU sistem |
|---|---|---|
| X-ray tuneli (parcel/customs) | Smiths Detection (HI-SCAN serije), Rapiscan (ORION 920CX, 638DV, Gemini 100100), Leidos (VACIS, Reveal), Astrophysics (XIS-6545N) | Kriticno: traziti status/alarm I/O i mrezni interfejs da gateway preuzme telemetriju i komande bez PLC sloja. |
| Diverteri i sortacija (shoe/pop-up/crossbelt) | Honeywell Intelligrated (Sliding Shoe, Wheel Divert), Vanderlande (POSISORTER, Steerable Wheel), Interroll (MX-V/MX-H) | Direktno pokriva profile `shoe` i `pop-up`; kontrola kroz MCU I/O + pogonske drajvere. |
| Barcode/OCR i tuneli skeniranja | Cognex (DataMan 470/390), Datalogic (AV7000 8K/12K), Zebra (FS42/FS20) | Pogodno za track-and-trace i OCR; integracija preko Ethernet i industrijskih protokola prema gateway-u. |
| Dimenzionisanje i merenje mase (DWS) | Mettler Toledo (dynamic parcel weighing, TLD950), Bizerba (dynamic checkweighers) | Potrebno za revenue protection i tacan routing; obavezno API/stream za vezu ka servisima. |
| Pogoni i motion | Danfoss (VLT FC 301/302), SEW-EURODRIVE (MOVI-C), ABB (conveyor drives), Yaskawa (Sigma-7) | MCU moze upravljati preko fieldbus/digital komandi; birati drajvere sa Safety funkcijama i dijagnostikom. |
| Pneumatika i ventilski blokovi | Festo (actuators, VTUX), SMC (cylinders, FRL) | Dobro za pop-up/wheel mehaniku; traziti ventilske terminale sa decentralizovanim I/O. |
| Safety oprema | Pilz (PNOZcompact/PNOZ), Banner (Type 4 safety light curtains) | Safety neka ostane hardverski nezavisan od MCU logike (safety relay + light curtains). |
| Industrijska mreza i edge gateway | Moxa (EDS serija), Cisco (IE 4010), Red Lion (FlexEdge) | Potrebno za segmentaciju OT mreze, visoku dostupnost i protokolnu konverziju ka MQTT/gRPC sloju. |
| MCU platforme (za zone kontrolere) | ST (STM32 Nucleo, STM32Cube), NXP (i.MX RT serija) | Najbolji fit sa postojecim repo tragovima (`nucleo_dev`); dobro pokriva real-time + industrijske interfejse. |

## Preporuceni shortlist za RFQ (faza 1)
1. X-ray: Rapiscan, Smiths Detection, Leidos.
2. Diverteri/sortacija: Honeywell Intelligrated, Vanderlande.
3. Skeneri/OCR: Datalogic, Cognex.
4. DWS: Mettler Toledo.
5. Motion/pneumatika: Danfoss + SEW + Festo (uz SMC kao alternativu).
6. Safety/network: Pilz + Moxa.
7. MCU platforma: ST STM32 kao primarno, NXP i.MX RT kao alternativa za zahtevnije zone.

## Prioritet po budzetu (MVP / Production / Enterprise)
| Nivo | Tipicna primena | Preporuceni dobavljaci | Gde stedeti | Gde ne stedeti |
|---|---|---|---|---|
| MVP (pilot 1 linija) | Validacija toka i integracije | X-ray: Rapiscan ili Astrophysics; Sortacija: Interroll; OCR: Zebra + jedan Cognex kanal; Motion: Danfoss; MCU: ST Nucleo | Redundansa, broj kamera po zoni, manje napredna analitika | Safety (Pilz/Banner), mrezna segmentacija, servisni ugovor za X-ray |
| Production (2-5 linija) | Stabilan rad u jednom sortnom centru | X-ray: Rapiscan + Smiths (drugi lot); Sortacija: Honeywell/Vanderlande; OCR: Datalogic + Cognex; DWS: Mettler Toledo | Vendor lock-in (drzati minimum 2 dobavljaca po kriticnoj kategoriji) | SLA za rezervne delove, citljivost OCR/barcode, MTBF pogona |
| Enterprise (vise centara) | Visok throughput i regionalna mreza centara | X-ray: Smiths + Leidos; Sortacija: Vanderlande + Honeywell; OCR: Datalogic + Cognex high-end; Network: Cisco IE + Moxa | Samo na nekriticnim perifernim modulima | Redundansa mreze/gateway-a, standardizacija interfejsa, globalni support ugovori |

## Minimalni tehnicki zahtevi po kategoriji (RFQ baseline)
Napomena: vrednosti su inicijalni prag za RFQ i treba ih potvrditi po finalnom layout-u i kapacitetu centra.

| Kategorija | Minimum (obavezno) | Target (pozeljno) | Operativa i SLA |
|---|---|---|---|
| X-ray tunel | 1x Ethernet + min 8 DI/8 DO (alarm/status/start-stop), API ili event export, integracija bez PLC zavisnosti | Redundant Ethernet, health telemetry, remote diagnostics | On-site response <= 8h, kriticni delovi <= 48h |
| Diverteri/sortacija | Komandna latencija <= 50 ms (MCU->aktuator), min 4.000 item/h po liniji, fault status po zoni | 8.000-12.000 item/h, prediktivna dijagnostika, hot-swap moduli | MTBF i plan servisiranja kvartalno |
| Barcode/OCR | Read rate >= 99.5% (1D/2D), OCR >= 98.0%, rezultat dostupan <= 100 ms | Read rate >= 99.8%, OCR >= 99.0%, automatski retake logic | Zamena kamere/skenera <= 30 min po stanici |
| DWS (dim/tezina) | Tacnost mase +/- 20 g (0.1-30 kg), timestamp i event ID, Ethernet export | Tacnost +/- 10 g, auto-kalibracija, audit trail | Godisnja verifikacija i zapisnik kalibracije |
| Pogoni i motion | STO (min SIL2/PLd), brzinske reference preko Ethernet ili DIO, fault code export | Dvokanalni safety, energy monitoring, condition monitoring | Kriticni VFD rezervni u lokalnom stock-u |
| Pneumatika | Ventilski blok sa feedback signalom, odziv aktuacije <= 30 ms, ciklusni vek deklarisan | Decentralizovan I/O terminal, > 20M ciklusa za kriticne ventile | PM interval i set zaptivki na stanju |
| Safety | Type 4 light curtain ili ekvivalent, safety relay nezavisan od MCU aplikacije, E-stop lanac Cat 3/4 | Safety dijagnostika sa centralnim logovanjem i test sekvencama | Godisnji safety proof test + kvartalna inspekcija |
| Industrijska mreza/gateway | Managed switch (VLAN, QoS, RSTP), dual PSU, -20..60C | PRP/HSR ili ring failover < 50 ms, out-of-band management | NMS alarmiranje i 24/7 monitoring |
| MCU kontroler | ARM MCU klase STM32/NXP RT, watchdog, brown-out, min 2 UART + GPIO banke + Ethernet, ciklus kontrole 10 ms | Dual Ethernet, secure boot, OTA update, hardware timestamping | Firmware release ciklus i rollback procedura |

## Scenariji opterecenja po liniji (4k / 8k / 12k item/h)
| Scenario | Kapacitet po liniji | Predlog arhitekture | Budzetski nivo |
|---|---|---|---|
| S1 | 4.000 item/h | 1x X-ray, 1x OCR tunel, 1x DWS tacka, single gateway, MCU ciklus 10 ms | MVP |
| S2 | 8.000 item/h | 1-2x X-ray (u zavisnosti od mix-a), 2x OCR zone, 1-2x DWS, gateway HA (active/standby), MCU ciklus 5 ms | Production |
| S3 | 12.000 item/h | 2x X-ray + bypass, multi-OCR (3+ ugla), dual DWS, dual gateway + ring/PRP mreza, MCU ciklus 2-5 ms | Enterprise |

## Varijanta tehnickih pragova po scenariju (RFQ)
| Kategorija | S1: 4k item/h | S2: 8k item/h | S3: 12k item/h |
|---|---|---|---|
| X-ray | Event/status latency <= 250 ms, min 8 DI/8 DO, 1x Ethernet | Event/status latency <= 150 ms, min 16 DI/16 DO, dual NIC pozeljan | Event/status latency <= 100 ms, dual Ethernet obavezan, N+1 raspolozivost |
| Diverteri/sortacija | Aktuacija <= 50 ms, miss-divert <= 0.5% | Aktuacija <= 35 ms, miss-divert <= 0.2% | Aktuacija <= 25 ms, miss-divert <= 0.1%, prediktivna dijagnostika |
| Barcode/OCR | Read >= 99.5%, OCR >= 98.0%, rezultat <= 100 ms | Read >= 99.7%, OCR >= 98.5%, rezultat <= 70 ms | Read >= 99.85%, OCR >= 99.0%, rezultat <= 50 ms, multi-view |
| DWS | Tacnost +/- 20 g, sync sa tracking ID <= 100 ms | Tacnost +/- 15 g, sync <= 75 ms | Tacnost +/- 10 g, sync <= 50 ms, audit trail obavezan |
| Pogoni/motion | STO SIL2/PLd, update reference <= 20 ms | STO + safety dijagnostika, update <= 10 ms | Dvokanalni safety, update <= 5 ms, condition monitoring obavezan |
| Mreza/gateway | VLAN + QoS + RSTP, failover <= 1 s | Ring failover <= 200 ms, dual PSU + dual uplink | PRP/HSR ili ekvivalent <= 50 ms, out-of-band management |
| MCU kontroler | Ciklus kontrole 10 ms, watchdog, brown-out | Ciklus 5 ms, secure boot pozeljan, OTA kontrolisano | Ciklus 2-5 ms, secure boot + OTA + rollback obavezno, dual Ethernet pozeljan |

## RFQ checklist (obavezno traziti)
- Podrzani interfejsi: Ethernet, digital I/O, opcioni Modbus TCP/Profinet/EtherNet/IP.
- Latencija i maksimalni takt ciklusa za komande/status.
- Dijagnostika i event log (lokalno + remote).
- Bezbednosni sertifikati (CE/IEC), a za X-ray relevantne regulatorne uskladjenosti prema trzistu instalacije.
- MTBF, rezervni delovi, servisni SLA i vreme isporuke.
- Lokalna podrska i dostupnost servisera u regionu.
- Jasna granica odgovornosti izmedju mehanike, sigurnosti i softverske integracije.

## Referentni linkovi (zvanicni)
- Smiths Detection: https://www.smithsdetection.com/
- Rapiscan Systems: https://www.rapiscansystems.com/en/products
- Leidos (VACIS/Reveal): https://www.leidos.com/products/vacis
- Astrophysics: https://astrophysicsinc.com/products/
- Honeywell Intelligrated sortation: https://automation.honeywell.com/us/en/products/warehouse-automation/solutions-by-technology/sortation-systems
- Vanderlande parcel systems: https://www.vanderlande.com/parcel/systems/
- Interroll sorters: https://www.interroll.com/products/sorters
- Cognex DataMan: https://www.cognex.com/products/barcode-readers/fixed-mount-barcode-readers
- Datalogic AV7000: https://cdn.datalogic.com/eng/retail-manufacturing-transportation-logistics/stationary-industrial-scanners/av7000-pd-709.html
- Zebra FS serija: https://www.zebra.com/us/en/products/industrial-machine-vision-fixed-scanners/fixed-industrial-barcode-scanners.html
- Mettler Toledo DWS: https://www.mt.com/us/en/home/products/Transport_and_Logistics_Solutions/weighing/dynamic-parcel-weighing.html
- Danfoss FC 301/302: https://www.danfoss.com/en-us/products/dds/low-voltage-drives/vlt-drives/vlt-automationdrive-fc-301-fc-302/
- SEW MOVI-C: https://www.sew-eurodrive.com/products/movi-c_drive_technology/movi-c_drive_technology.html
- ABB conveyors/drives: https://www.abb.com/global/en/areas/motion/applications/conveyors
- Festo actuators: https://www.festo.com/us/en/c/products/actuators-and-drives-id_pim5/
- SMC pneumatics: https://www.smcusa.com/
- Pilz safety relays: https://www.pilz.com/en-US/products/relay-modules/safety-relays-protection-relays
- Banner safety light curtains: https://www.bannerengineering.com/us/en/products/machine-safety/safety-light-curtains.html
- Moxa industrial switches: https://www.moxa.com/en/products/industrial-network-infrastructure/ethernet-switches
- Cisco IE 4010: https://www.cisco.com/c/en/us/products/switches/industrial-ethernet-4010-series-switches/index.html
- Red Lion FlexEdge: https://www.redlion.net/flexedge
- ST STM32 Nucleo: https://www.st.com/content/st_com/en/products/ecosystems/stm32-open-development-environment/stm32-nucleo.html
- NXP i.MX RT: https://www.nxp.com/products/processors-and-microcontrollers/arm-microcontrollers/i-mx-rt-crossover-mcus:IMX-RT-SERIES
