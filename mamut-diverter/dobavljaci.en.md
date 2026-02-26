# Equipment Suppliers for mamut-diverter (MCU Architecture)

## Context
This proposal assumes a no-PLC architecture: zone control runs on microcontrollers (firmware), and integration is handled through gateway/backend services. The focus is on industrial suppliers that support Ethernet/DIO integration.

## Suppliers by Category
| Category | Suppliers and examples | Fit for MCU system |
|---|---|---|
| X-ray tunnels (parcel/customs) | Smiths Detection (HI-SCAN series), Rapiscan (ORION 920CX, 638DV, Gemini 100100), Leidos (VACIS, Reveal), Astrophysics (XIS-6545N) | Critical: require status/alarm I/O and network interfaces so gateway can handle telemetry/commands without a PLC layer. |
| Diverters and sortation (shoe/pop-up/crossbelt) | Honeywell Intelligrated (Sliding Shoe, Wheel Divert), Vanderlande (POSISORTER, Steerable Wheel), Interroll (MX-V/MX-H) | Directly matches `shoe` and `pop-up` profiles; control via MCU I/O and drive controllers. |
| Barcode/OCR and scanning tunnels | Cognex (DataMan 470/390), Datalogic (AV7000 8K/12K), Zebra (FS42/FS20) | Suitable for track-and-trace and OCR; Ethernet and industrial-protocol integration toward gateway. |
| Dimensioning and weighing (DWS) | Mettler Toledo (dynamic parcel weighing, TLD950), Bizerba (dynamic checkweighers) | Required for revenue protection and accurate routing; API/stream output to services is mandatory. |
| Drives and motion | Danfoss (VLT FC 301/302), SEW-EURODRIVE (MOVI-C), ABB (conveyor drives), Yaskawa (Sigma-7) | MCU can control through fieldbus/digital commands; choose drives with safety functions and diagnostics. |
| Pneumatics and valve blocks | Festo (actuators, VTUX), SMC (cylinders, FRL) | Good fit for pop-up/wheel mechanics; require valve terminals with decentralized I/O. |
| Safety equipment | Pilz (PNOZcompact/PNOZ), Banner (Type 4 safety light curtains) | Safety should remain hardware-independent from MCU application logic (safety relays + light curtains). |
| Industrial networking and edge gateway | Moxa (EDS series), Cisco (IE 4010), Red Lion (FlexEdge) | Needed for OT segmentation, high availability, and protocol conversion toward MQTT/gRPC layers. |
| MCU platforms (zone controllers) | ST (STM32 Nucleo, STM32Cube), NXP (i.MX RT series) | Best fit with current repo traces (`nucleo_dev`); strong real-time and industrial interface support. |

## Recommended Shortlist for RFQ (Phase 1)
1. X-ray: Rapiscan, Smiths Detection, Leidos.
2. Diverters/sortation: Honeywell Intelligrated, Vanderlande.
3. Scanners/OCR: Datalogic, Cognex.
4. DWS: Mettler Toledo.
5. Motion/pneumatics: Danfoss + SEW + Festo (SMC as alternative).
6. Safety/network: Pilz + Moxa.
7. MCU platform: ST STM32 as primary, NXP i.MX RT as alternative for demanding zones.

## Budget Priority (MVP / Production / Enterprise)
| Level | Typical use | Recommended suppliers | Where to save | Where not to save |
|---|---|---|---|---|
| MVP (single pilot line) | Validate flow and integration | X-ray: Rapiscan or Astrophysics; Sortation: Interroll; OCR: Zebra + one Cognex lane; Motion: Danfoss; MCU: ST Nucleo | Redundancy, camera count per zone, advanced analytics | Safety (Pilz/Banner), network segmentation, X-ray service contract |
| Production (2-5 lines) | Stable operation in one sort center | X-ray: Rapiscan + Smiths (second lot); Sortation: Honeywell/Vanderlande; OCR: Datalogic + Cognex; DWS: Mettler Toledo | Vendor lock-in (keep at least 2 suppliers in each critical category) | Spare-parts SLA, OCR/barcode readability, drive MTBF |
| Enterprise (multiple centers) | High throughput and regional network of hubs | X-ray: Smiths + Leidos; Sortation: Vanderlande + Honeywell; OCR: Datalogic + Cognex high-end; Network: Cisco IE + Moxa | Only on non-critical peripheral modules | Network/gateway redundancy, interface standardization, global support contracts |

## Minimum Technical Requirements by Category (RFQ Baseline)
Note: values are initial RFQ thresholds and should be validated against final layout and site capacity.

| Category | Minimum (required) | Target (preferred) | Operations and SLA |
|---|---|---|---|
| X-ray tunnel | 1x Ethernet + at least 8 DI/8 DO (alarm/status/start-stop), API or event export, no PLC dependency | Redundant Ethernet, health telemetry, remote diagnostics | On-site response <= 8h, critical spares <= 48h |
| Diverters/sortation | Command latency <= 50 ms (MCU->actuator), at least 4,000 item/h per line, fault status by zone | 8,000-12,000 item/h, predictive diagnostics, hot-swap modules | MTBF tracking and quarterly service plan |
| Barcode/OCR | Read rate >= 99.5% (1D/2D), OCR >= 98.0%, result available <= 100 ms | Read rate >= 99.8%, OCR >= 99.0%, automatic retake logic | Camera/scanner replacement <= 30 min per station |
| DWS (dimension/weight) | Weight accuracy +/- 20 g (0.1-30 kg), timestamp and event ID, Ethernet export | +/- 10 g accuracy, auto-calibration, audit trail | Annual verification and calibration record |
| Drives and motion | STO (at least SIL2/PLd), speed references via Ethernet or DIO, fault code export | Dual-channel safety, energy monitoring, condition monitoring | Keep critical VFD spare locally |
| Pneumatics | Valve block with feedback, actuation response <= 30 ms, declared cycle life | Decentralized I/O terminal, > 20M cycles for critical valves | PM interval and seal kit in stock |
| Safety | Type 4 light curtain or equivalent, safety relay independent from MCU app, E-stop chain Cat 3/4 | Safety diagnostics with centralized logging and test sequences | Annual safety proof test + quarterly inspection |
| Industrial network/gateway | Managed switch (VLAN, QoS, RSTP), dual PSU, -20..60C | PRP/HSR or ring failover < 50 ms, out-of-band management | NMS alerting and 24/7 monitoring |
| MCU controller | ARM MCU class STM32/NXP RT, watchdog, brown-out, at least 2 UART + GPIO banks + Ethernet, 10 ms control cycle | Dual Ethernet, secure boot, OTA update, hardware timestamping | Firmware release cadence and rollback procedure |

## Throughput Scenarios per Line (4k / 8k / 12k item/h)
| Scenario | Capacity per line | Architecture suggestion | Budget level |
|---|---|---|---|
| S1 | 4,000 item/h | 1x X-ray, 1x OCR tunnel, 1x DWS point, single gateway, MCU cycle 10 ms | MVP |
| S2 | 8,000 item/h | 1-2x X-ray (depending on mix), 2x OCR zones, 1-2x DWS, gateway HA (active/standby), MCU cycle 5 ms | Production |
| S3 | 12,000 item/h | 2x X-ray + bypass, multi-OCR (3+ angles), dual DWS, dual gateway + ring/PRP network, MCU cycle 2-5 ms | Enterprise |

## Technical Threshold Variant by Scenario (RFQ)
| Category | S1: 4k item/h | S2: 8k item/h | S3: 12k item/h |
|---|---|---|---|
| X-ray | Event/status latency <= 250 ms, at least 8 DI/8 DO, 1x Ethernet | Event/status latency <= 150 ms, at least 16 DI/16 DO, dual NIC preferred | Event/status latency <= 100 ms, dual Ethernet mandatory, N+1 availability |
| Diverters/sortation | Actuation <= 50 ms, miss-divert <= 0.5% | Actuation <= 35 ms, miss-divert <= 0.2% | Actuation <= 25 ms, miss-divert <= 0.1%, predictive diagnostics |
| Barcode/OCR | Read >= 99.5%, OCR >= 98.0%, result <= 100 ms | Read >= 99.7%, OCR >= 98.5%, result <= 70 ms | Read >= 99.85%, OCR >= 99.0%, result <= 50 ms, multi-view |
| DWS | +/- 20 g accuracy, tracking ID sync <= 100 ms | +/- 15 g accuracy, sync <= 75 ms | +/- 10 g accuracy, sync <= 50 ms, audit trail mandatory |
| Drives/motion | STO SIL2/PLd, reference update <= 20 ms | STO + safety diagnostics, update <= 10 ms | Dual-channel safety, update <= 5 ms, condition monitoring mandatory |
| Network/gateway | VLAN + QoS + RSTP, failover <= 1 s | Ring failover <= 200 ms, dual PSU + dual uplink | PRP/HSR or equivalent <= 50 ms, out-of-band management |
| MCU controller | 10 ms control cycle, watchdog, brown-out | 5 ms cycle, secure boot preferred, controlled OTA | 2-5 ms cycle, secure boot + OTA + rollback mandatory, dual Ethernet preferred |

## RFQ Checklist (Mandatory)
- Supported interfaces: Ethernet, digital I/O, optional Modbus TCP/Profinet/EtherNet/IP.
- Latency and maximum control/status cycle rate.
- Diagnostics and event logs (local + remote).
- Safety certifications (CE/IEC), and X-ray regulatory compliance per target market.
- MTBF, spare parts, service SLA, and lead times.
- Local support and regional service engineer availability.
- Clear responsibility boundaries across mechanics, safety, and software integration.

## Reference Links (Official)
- Smiths Detection: https://www.smithsdetection.com/
- Rapiscan Systems: https://www.rapiscansystems.com/en/products
- Leidos (VACIS/Reveal): https://www.leidos.com/products/vacis
- Astrophysics: https://astrophysicsinc.com/products/
- Honeywell Intelligrated sortation: https://automation.honeywell.com/us/en/products/warehouse-automation/solutions-by-technology/sortation-systems
- Vanderlande parcel systems: https://www.vanderlande.com/parcel/systems/
- Interroll sorters: https://www.interroll.com/products/sorters
- Cognex DataMan: https://www.cognex.com/products/barcode-readers/fixed-mount-barcode-readers
- Datalogic AV7000: https://cdn.datalogic.com/eng/retail-manufacturing-transportation-logistics/stationary-industrial-scanners/av7000-pd-709.html
- Zebra FS series: https://www.zebra.com/us/en/products/industrial-machine-vision-fixed-scanners/fixed-industrial-barcode-scanners.html
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
