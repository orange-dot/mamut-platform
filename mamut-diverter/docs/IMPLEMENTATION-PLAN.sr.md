# mamut-diverter План имплементације

> **POC стратегија: Прво симулација**
>
> Ово је POC без хардвера. Сваки физички уређај (скретница, сензор, мотор транспортера, баркод скенер) замењен је софтверским симулатором који говори истим MQTT протоколом као што би стварни фирмвер. MQTT уговор о темама/порукама је граница — сервиси изнад ње не знају и не занима их да ли комуницирају са Python симулатором или стварним MCU фирмвером. Када стигне хардвер, замењујемо симулаторе стварним фирмвером. Core логика остаје нетакнута.

---

## Архитектура: Граница симулације

```
┌─────────────────────────────────────────────────┐
│              Контролна табла (React)              │
│                   gRPC-web                       │
├─────────────────────────────────────────────────┤
│           Rust сервиси (gRPC/tonic)              │
│  controller · gateway · wms-bridge · postal      │
├──────────────────── MQTT ────────────────────────┤  ← граница уговора
│         Симулатори уређаја (Python)               │
│  sim-conveyor · sim-diverter · sim-scanner       │
│  sim-sensor · sim-safety                         │
└─────────────────────────────────────────────────┘
         ↕ (будућност: замена за стварни фирмвер)
    ┌──────────────┐
    │  C фирмвер   │  ← одложено за фазу хардвера
    │  на стварном  │
    │     MCU-у     │
    └──────────────┘
```

Именовање MQTT тема и формати protobuf порука дефинисани у Фази 1 су **непроменљив уговор**. Све изнад и испод те линије је заменљиво.

---

## Фаза 1: Генерисање Proto кода и основа транспорта

**Циљ**: Успоставити генерисане типове, MQTT уговор о порукама и магистралу догађаја од којих зависе сви сервиси.

### 1.0 — Оперативни план Фазе 1 (спринт: 10 радних дана)

**Обим (scope)**:
- 1А Proto генерација (`mamut-proto` crate + `tools/proto-gen.sh`)
- 1Б MQTT уговор (`spec/protocols/mqtt-contract.md`)
- 1В Транспорт (`mamut-transport` event bus + config)
- Минимална документација која отклања stub-ове за овај слој

**Definition of Done (Фаза 1 је готова када):**
- `make proto` пролази и `cargo build -p mamut-proto` пролази локално и у CI.
- `cargo test -p mamut-transport` има покривен бар `publish/subscribe` и учитавање конфигурације.
- `spec/protocols/mqtt-contract.md` дефинише стабло тема, типове порука, QoS/LWT и примере payload-а.
- `spec/system/data-flow.md` описује MQTT↔gRPC токове без недоречености.
- Нема преосталих stub-ова у датотекама које припадају Фази 1.

**Редослед имплементације (дан по дан):**
| Дан | Пакет | Главни излаз |
|---|---|---|
| Д1 | Подешавање `mamut-proto` crate-а | `Cargo.toml`, `build.rs`, `src/lib.rs` scaffold |
| Д2 | Proto compile pipeline | Генерација свих `proto/mamut/*.proto` и успешан build |
| Д3 | `tools/proto-gen.sh` + Make интеграција | `make proto` ради без ручних корака |
| Д4 | MQTT уговор v1 | Нацрт `mqtt-contract.md` са темама/типовима/QoS |
| Д5 | Преглед уговора и закључавање v1 | Финална спецификација + примери порука |
| Д6 | `mamut-transport` event bus | `event_bus.rs` са тестовима за round-trip |
| Д7 | `mamut-transport` config | `config.rs` + TOML parsing тестови |
| Д8 | `spec/system/data-flow.md` | Документација канала и токова порука |
| Д9 | Интеграциони smoke run | `make proto`, `cargo test -p mamut-transport` |
| Д10 | Харденинг и PR | cleanup, lint, финални checklist, PR опис |

**Стартни backlog (тикети):**
- `F1-01` Креирати `services/crates/mamut-proto` и увести у workspace.
- `F1-02` Имплементирати `build.rs` за `tonic-build` над свим protobuf датотекама.
- `F1-03` Заменити `tools/proto-gen.sh` stub-ом који покреће генерацију.
- `F1-04` Написати `spec/protocols/mqtt-contract.md` (v1, immutable contract).
- `F1-05` Имплементирати `mamut-transport/src/event_bus.rs`.
- `F1-06` Имплементирати `mamut-transport/src/config.rs`.
- `F1-07` Заменити `spec/system/data-flow.md` stub конкретним токовима.
- `F1-08` Додати/поправити тестове и верификационе команде у CI.

**Ризици и мере умањења:**
- Непотпуна proto генерација: додати тест који проверава да се све `.proto` датотеке компајлирају у једном пролазу.
- MQTT договор без довољно детаља: обавезно укључити примере за `status`, `command`, `alarm`, `telemetry`.
- Рана регресија у transport crate-у: прво тестови за event bus, па тек онда интеграција у сервисе.

**Критеријум за улазак у Фазу 2:**
- Све ставке `F1-01` до `F1-08` означене као завршене.
- Команде: `make proto`, `cargo test -p mamut-transport`, `cargo build -p mamut-proto` пролазе.
- Документација и код Фазе 1 одобрени у једном PR-у.

### 1А — Генерисање Proto кода (средње)

**Документација**: Написати `spec/protocols/scada-interface.md` са gRPC мапом сервиса; ажурирати `docs/en/api/grpc.md` и `docs/sr/api/grpc.md` референцом сервиса генерисаном из proto коментара.

**Код**:
- Креирати `services/crates/mamut-proto/` — нови crate са `build.rs` користећи `tonic-build` за компајлирање свих 8 `.proto` датотека под `proto/mamut/`
- Додати `mamut-proto` у workspace чланове у `services/Cargo.toml`
- Додати workspace зависности: `tonic-build`, `prost-build`
- Имплементирати `tools/proto-gen.sh` за позивање `cargo build -p mamut-proto` (замењује stub)
- Реекспортовати генерисане типове из `mamut-proto/src/lib.rs`

**Датотеке за креирање/измену**:
- `services/crates/mamut-proto/Cargo.toml` (ново)
- `services/crates/mamut-proto/build.rs` (ново)
- `services/crates/mamut-proto/src/lib.rs` (ново)
- `services/Cargo.toml` (додати члан + зависности)
- `tools/proto-gen.sh` (заменити stub)

**Верификација**: `make proto && cargo build -p mamut-proto` — генерисани Rust типови се компајлирају.

### 1Б — MQTT уговор о порукама (средње)

**Документација**: Написати `spec/protocols/mqtt-contract.md` — **једини извор истине** за MQTT стабло тема и формате порука које и симулатори и будући фирмвер морају да прате:
- Стабло тема: `mamut/{site}/{device_type}/{device_id}/{msg_type}` (нпр. `mamut/sim/diverter/div-01/status`)
- Типови порука: извештаји о статусу (уређај→сервиси), команде (сервиси→уређај), телеметрија, аларми
- Серијализација: JSON за POC (protobuf-c за будући фирмвер)
- QoS нивои, retained поруке, LWT (Last Will & Testament) за online/offline статус уређаја

Ова спецификација је **непроменљив уговор** између сервиса и уређаја — симулатори га имплементирају сада, стварни фирмвер ће га имплементирати касније.

**Датотеке за креирање/измену**:
- `spec/protocols/mqtt-contract.md` (ново — критичан документ)

### 1В — Транспорт / магистрала догађаја (средње)

**Документација**: Написати `spec/system/data-flow.md` — документовати канале магистрале догађаја, именовање MQTT тема, gRPC стриминг обрасце.

**Код** — имплементирати `services/crates/mamut-transport/src/`:
- `lib.rs` — реекспорти модула
- `event_bus.rs` — ин-процесна магистрала догађаја користећи `tokio::sync::broadcast` за `SystemEvent`
- `config.rs` — учитавање TOML конфигурације користећи `serde` (grpc_listen, mqtt_broker адресе)

**Датотеке за измену**:
- `services/crates/mamut-transport/Cargo.toml` (додати зависности: mamut-core, mamut-proto, tokio, toml, serde)
- `services/crates/mamut-transport/src/lib.rs` (заменити stub)
- `services/crates/mamut-transport/src/event_bus.rs` (ново)
- `services/crates/mamut-transport/src/config.rs` (ново)
- `spec/system/data-flow.md` (заменити stub)

**Верификација**: `cargo test -p mamut-transport` — објави/претплати се round-trip догађаја.

---

## Фаза 2: Телеметрија, аларми и основа спецификација

**Циљ**: Заједничке компоненте потребне сваком сервису, плус основне спецификације.

### 2А — Телеметрија crate (средње)

**Документација**: Ажурирати `config/defaults/telemetry.toml` са пуном шемом; написати `docs/en/operations/deployment.md` са секцијом за мониторинг.

**Код** — имплементирати `services/crates/mamut-telemetry/src/`:
- `lib.rs` — init_tracing(), init_metrics() функције за подешавање
- `health.rs` — `HealthRegistry` структура која имплементира `TelemetryService::GetHealth` преко tonic-а
- `metrics.rs` — Prometheus-компатибилно бележење метрика + `StreamMetrics` серверски стриминг

**Датотеке за измену**:
- `services/crates/mamut-telemetry/Cargo.toml` (додати зависности: mamut-proto, tonic, tracing-subscriber, prometheus)
- `services/crates/mamut-telemetry/src/lib.rs`, `health.rs`, `metrics.rs`
- `config/defaults/telemetry.toml` (проширити)
- `docs/en/operations/deployment.md` (заменити stub)
- `docs/sr/operations/deployment.sr.md` (заменити stub)

**Верификација**: `cargo test -p mamut-telemetry` — здравствена провера враћа здрав статус, стриминг метрика ради.

### 2Б — Аларм crate (средње)

**Документација**: Написати `spec/system/safety-interlocks.md` — дефинисати озбиљности аларма, правила ескалације, ISA-18.2 животни циклус.

**Код** — имплементирати `services/crates/mamut-alarm/src/`:
- `lib.rs` — експорти модула
- `alarm_manager.rs` — `AlarmManager` структура: животни циклус покретање/потврда/гашење, меморијско складиште
- `service.rs` — tonic `AlarmService` имплементација (GetActiveAlarms, AcknowledgeAlarm, StreamAlarms)

**Датотеке за измену**:
- `services/crates/mamut-alarm/Cargo.toml`
- `services/crates/mamut-alarm/src/lib.rs`, `alarm_manager.rs`, `service.rs`
- `spec/system/safety-interlocks.md` (заменити stub)

**Верификација**: `cargo test -p mamut-alarm` — животни циклус покретање→потврда→гашење; стриминг испоручује нове аларме.

### 2В — Допуна спецификација (средње)

Написати суштински садржај за основне спецификације:
- `spec/system/architecture.md` — проширити са одговорностима слојева, топологијом постављања, матрицом комуникације. **Експлицитно документовати границу симулације**: MQTT је слој апстракције хардвера, симулатори живе испод њега, сервиси изнад.
- `spec/system/state-machines.md` — дефинисати стања зона (Мировање→Рад→Грешка→Заустављено) и стања скретница (Мировање→Активно→Грешка→Тестирање) са таблицама прелаза
- `spec/conveyor/zone-management.md` — животни циклус зоне, рампирање брзине, протокол примопредаје предмета
- `spec/conveyor/speed-control.md` — PID параметри, профили убрзања/успорења, јединице
- `spec/conveyor/belt-transport.md` — физички модел, типови трака, мониторинг затезања

**Датотеке за измену**: 5 spec датотека наведених горе (заменити stub-ове садржајем).

**Верификација**: Преглед унутрашње конзистентности са proto дефиницијама и постојећим diverter-interface.md.

---

## Фаза 3: Основни сервисни crate-ови

**Циљ**: Имплементирати crate-ове доменске логике које бинарне датотеке композирају. Сви crate-ови су агностични у односу на хардвер — конзумирају догађаје са магистрале догађаја и издају команде преко gRPC/MQTT. Никад не комуницирају директно са хардвером.

### 3А — Транспортер crate (средње)

**Код** — имплементирати `services/crates/mamut-conveyor/src/`:
- `lib.rs` — реекспорти
- `zone.rs` — `Zone` структура са машином стања (Мировање/Рад/Грешка/Заустављено), контрола брзине
- `service.rs` — tonic `ConveyorService` (GetZoneStatus, SetZoneSpeed, StreamZoneEvents)

**Датотеке за измену**:
- `services/crates/mamut-conveyor/Cargo.toml` (додати зависности: mamut-core, mamut-proto, mamut-transport, tonic, tokio)
- `services/crates/mamut-conveyor/src/lib.rs`, `zone.rs`, `service.rs`

**Верификација**: `cargo test -p mamut-conveyor` — прелази стања зоне, ограничења брзине спроведена, стриминг догађаја.

### 3Б — Идентификација crate (средње)

**Документација**: Написати `spec/identification/barcode-scanning.md` — типови баркодова, прагови поузданости, логика поновног покушаја; `spec/identification/item-tracking.md` — модел праћења.

**Код** — имплементирати `services/crates/mamut-identification/src/`:
- `lib.rs` — реекспорти
- `scanner.rs` — `ScannerService` управљање резултатима скенирања
- `tracker.rs` — `ItemTracker` мапирање item_id → zone_id са временским печатима
- `service.rs` — tonic `IdentificationService` + `TrackingService`

**Датотеке за измену**:
- `services/crates/mamut-identification/Cargo.toml`
- `services/crates/mamut-identification/src/lib.rs`, `scanner.rs`, `tracker.rs`, `service.rs`
- `spec/identification/barcode-scanning.md`, `spec/identification/item-tracking.md`

**Верификација**: `cargo test -p mamut-identification` — скенирање→праћење→упит позиције ради.

### 3В — WMS crate (мало)

**Документација**: Написати `spec/protocols/wms-interface.md` — протокол одлуке о сортирању, SAP/Manhattan обрасци адаптера.

**Код** — имплементирати `services/crates/mamut-wms/src/`:
- `lib.rs` — реекспорти
- `sort_engine.rs` — `SortEngine` примењује правила рутирања и враћа `SortDecision`
- `service.rs` — tonic `WmsService::GetSortDecision`

**Датотеке за измену**:
- `services/crates/mamut-wms/Cargo.toml`
- `services/crates/mamut-wms/src/lib.rs`, `sort_engine.rs`, `service.rs`
- `spec/protocols/wms-interface.md`

**Верификација**: `cargo test -p mamut-wms` — одлуке о сортирању се разрешавају за познате предмете.

### 3Г — Поштански crate (мало)

**Документација**: Написати `spec/protocols/postal-sorting-plans.md`, `spec/protocols/edi-formats.md`, `spec/protocols/customs-declarations.md`.

**Код** — имплементирати `services/crates/mamut-postal/src/`:
- `lib.rs` — реекспорти
- `sorting_plan.rs` — парсер поштанског плана сортирања (одредиште→смер мапирање)
- `customs.rs` — модел података царинске декларације

**Датотеке за измену**:
- `services/crates/mamut-postal/Cargo.toml`
- `services/crates/mamut-postal/src/lib.rs`, `sorting_plan.rs`, `customs.rs`
- 3 spec датотеке наведене горе

**Верификација**: `cargo test -p mamut-postal` — парсирање плана сортирања, round-trip царинских података.

---

## Фаза 4: Сервисне бинарне датотеке

**Циљ**: Повезати crate-ове у покренуте gRPC сервере. Gateway се повезује на MQTT брокер где симулатори (не хардвер) објављују поруке уређаја.

### 4А — mamut-gateway (велико)

**Документација**: Ажурирати `config/defaults/gateway.toml` са MQTT конфигурацијом тема, политиком поновног повезивања, TLS подешавањима.

**Код** — имплементирати `services/binaries/mamut-gateway/src/`:
- `main.rs` — tokio::main, учитавање конфигурације, покретање tonic сервера
- `mqtt.rs` — MQTT клијент (rumqttc) претплата на теме уређаја, објављивање команди. У POC-у ове поруке долазе од Python симулатора; gateway не зна разлику.
- `bridge.rs` — двосмерна MQTT↔gRPC транслација: поруке сензора/статуса уређаја → gRPC догађаји; gRPC команде → MQTT теме команди

**Зависности за додавање**: `rumqttc`, `mamut-proto`, `mamut-transport`, `mamut-telemetry`, `mamut-conveyor`, `mamut-diverter`

**Датотеке за измену**:
- `services/binaries/mamut-gateway/Cargo.toml`
- `services/binaries/mamut-gateway/src/main.rs`, `mqtt.rs`, `bridge.rs`
- `config/defaults/gateway.toml`

**Верификација**: `cargo run -p mamut-gateway -- --config config/defaults/` покреће се и слуша на :50051; здравствена крајња тачка одговара. Са покренутим MQTT брокером, претплаћује се на теме уређаја.

### 4Б — mamut-controller (велико)

**Документација**: Написати `docs/en/architecture/overview.md`, `docs/sr/architecture/overview.sr.md` — ток оркестрације контролера.

**Код** — имплементирати `services/binaries/mamut-controller/src/`:
- `main.rs` — tokio::main, tonic сервер са свим сервисима монтираним
- `orchestrator.rs` — главна контролна петља: прима догађаје са магистрале догађаја, координира активацију скретнице на основу скенирање→одлука сортирања→позиција зоне
- `config.rs` — учитавање конфигурације контролера

**Зависности**: сви доменски crate-ови + mamut-proto, mamut-transport, mamut-telemetry, mamut-alarm

**Датотеке за измену**:
- `services/binaries/mamut-controller/Cargo.toml`
- `services/binaries/mamut-controller/src/main.rs`, `orchestrator.rs`, `config.rs`
- `docs/en/architecture/overview.md`, `docs/sr/architecture/overview.sr.md`

**Верификација**: `cargo run -p mamut-controller` покреће се, монтира gRPC сервисе на :50052, здравствена провера пролази.

### 4В — mamut-wms-bridge (мало)

**Код** — имплементирати `services/binaries/mamut-wms-bridge/src/`:
- `main.rs` — tokio::main, покреће WmsService gRPC сервер
- `client.rs` — mock WMS клијент који враћа конфигурабилне одлуке о сортирању (нема екстерног WMS-а у POC-у)

**Датотеке за измену**:
- `services/binaries/mamut-wms-bridge/Cargo.toml`
- `services/binaries/mamut-wms-bridge/src/main.rs`, `client.rs`

### 4Г — mamut-postal-bridge (мало)

**Код** — имплементирати `services/binaries/mamut-postal-bridge/src/`:
- `main.rs` — tokio::main, покреће сервис интеграције поштанског/царинског система
- `edi_client.rs` — mock EDI обрађивач који враћа припремљене одговоре (нема стварног EDI-а у POC-у)

**Датотеке за измену**:
- `services/binaries/mamut-postal-bridge/Cargo.toml`
- `services/binaries/mamut-postal-bridge/src/main.rs`, `edi_client.rs`

**Верификација (4В+4Г)**: Обе бинарне датотеке се покрећу, одговарају на здравствене провере. `make test-services` пролази.

---

## Фаза 5: Симулатори уређаја

**Циљ**: Изградити Python процесе који емулирају сваки физички уређај преко MQTT-а, пратећи уговор из `spec/protocols/mqtt-contract.md`. Ови симулатори **јесу** хардвер за POC.

### 5А — Оквир симулатора и симулација транспортера (средње)

**Документација**: Написати `spec/simulation/simulator-framework.md` — документовати архитектуру симулатора: како се сваки сим уређај повезује на MQTT, објављује статус, прима команде и моделира тајминг/грешке.

**Код**:
- `sim/mamut_sim/mqtt_client.py` (ново) — дељени MQTT клијент wrapper (paho-mqtt), повезује се на брокер, пружа помоћне функције за објављивање/претплату са шаблонирањем тема из уговора
- `sim/mamut_sim/base_device.py` (ново) — `BaseDevice` класа: MQTT конекција, учитавање конфигурације, скелет машине стања, периодично објављивање статуса, регистрација обрађивача команди, LWT подешавање
- `sim/mamut_sim/conveyor.py` — `SimulatedConveyor`: машина стања зоне (Мировање→Рад→Грешка→Заустављено), конфигурабилна брзина траке, праћење предмета у зони, објављује статус зоне, одговара на команде брзине/покретања/заустављања
- `sim/mamut_sim/sensor.py` (ново) — `SimulatedSensor`: емулација фото-око сензора, генерише догађаје детектован-предмет/уклоњен-предмет на тајмерима, конфигурабилан debounce, објављује на теме сензора

**Зависности за додавање у `sim/pyproject.toml`**: `paho-mqtt`

**Датотеке за креирање/измену**:
- `sim/mamut_sim/mqtt_client.py` (ново)
- `sim/mamut_sim/base_device.py` (ново)
- `sim/mamut_sim/conveyor.py` (заменити stub)
- `sim/mamut_sim/sensor.py` (ново)
- `sim/pyproject.toml` (додати paho-mqtt)
- `spec/simulation/simulator-framework.md` (ново)

**Верификација**: `pytest sim/` — `SimulatedConveyor` објављује статус на MQTT, одговара на команде брзине, прелази зона раде.

### 5Б — Симулатори скретнице и скенера (средње)

**Код**:
- `sim/mamut_sim/diverter.py` — `SimulatedDiverter`: моделира кашњење активације (конфигурабилно ms), промену смера, убацивање грешака (насумично или скриптовано), одговор на самотестирање. Објављује статус скретнице, одговара на команде активације/деактивације/самотестирања. Подржава сва 3 профила (MDR, popup, shoe) преко конфигурације — различити параметри тајминга, исти MQTT интерфејс.
- `sim/mamut_sim/scanner.py` — `SimulatedScanner`: генерише догађаје скенирања баркода када се покрене (догађајем предмет-у-зони или тајмером), конфигурабилна поузданост скенирања и стопа промашаја, објављује резултате скенирања на теме идентификације

**Датотеке за измену**:
- `sim/mamut_sim/diverter.py` (заменити stub)
- `sim/mamut_sim/scanner.py` (заменити stub)

**Верификација**: `pytest sim/` — скретница се активира/деактивира са коректним тајмингом, скенер генерише догађаје скенирања са конфигурабилном поузданошћу.

### 5В — Симулатор безбедности и покретач уређаја (средње)

**Код**:
- `sim/mamut_sim/safety.py` (ново) — `SimulatedSafety`: моделира хитно заустављање, проверу блокада (зона мора бити заустављена пре активације скретнице), објављивање heartbeat-а надзорног тајмера. Дозвољава убацивање услова грешке за тестирање токова аларма.
- `sim/mamut_sim/launcher.py` (ново) — чита layout YAML, инстанцира све симулиране уређаје за тај распоред, повезује их све на MQTT брокер, управља животним циклусом (покретање/заустављање/статус). CLI: `python -m mamut_sim.launcher --layout sim/layouts/test-bench.yaml --broker localhost:1883`
- `sim/mamut_sim/__main__.py` (ново) — улазна тачка за `python -m mamut_sim`

**Конфигурација**:
- `config/defaults/simulator.toml` (ново) — конфигурација специфична за симулатор: множилац тајминга (за убрзано извршавање), вероватноћа убацивања грешке, подразумевани профили уређаја
- Ажурирати layout YAML-ове (`sim/layouts/test-bench.yaml`, `postal-center.yaml`, `customs.yaml`) са пуним дефиницијама уређаја: свака зона наводи своје сензоре, скретнице, скенере са конфигурацијом

**Датотеке за креирање/измену**:
- `sim/mamut_sim/safety.py` (ново)
- `sim/mamut_sim/launcher.py` (ново)
- `sim/mamut_sim/__main__.py` (ново)
- `sim/layouts/test-bench.yaml` (проширити)
- `sim/layouts/postal-center.yaml` (проширити)
- `sim/layouts/customs.yaml` (проширити)
- `config/defaults/simulator.toml` (ново)

**Верификација**: `python -m mamut_sim --layout sim/layouts/test-bench.yaml` покреће све симулиране уређаје, објављује heartbeat-ове на MQTT. `pytest sim/` — безбедносне блокаде спречавају активацију скретнице у зони са грешком.

### 5Г — Спецификације профила скретница (мало)

Написати спецификације уређаја које симулатори моделирају и које ће будући фирмвер имплементирати:
- `spec/diverter/profile-mdr.md` — MDR тајминг, кодови грешака, секвенца активације
- `spec/diverter/profile-popup.md` — Pop-up точкови тајминг, кодови грешака
- `spec/diverter/profile-shoe.md` — Клизни елементи тајминг, кодови грешака
- `spec/identification/ocr-labels.md` — OCR протокол интеграције (симулиран као варијанта скенера)

---

## Фаза 6: Системска симулација и сценарији

**Циљ**: End-to-end оркестрација сценарија — предмети који теку кроз симулирано постројење, тестирајући комплетан стек од симулатора преко сервиса до контролне табле.

### 6А — Модел распореда и мотор тока предмета (средње)

**Код**:
- `sim/mamut_sim/layout.py` — YAML учитавач распореда, гради усмерени граф зона и скретница, разрешава путање рутирања
- `sim/mamut_sim/item.py` (ново) — `SimulatedItem`: представља пакет са баркодом, одредиштем, тренутном позицијом. Креће се кроз зоне на основу брзине траке, покреће догађаје сензора при уласку/изласку из зоне.
- `sim/mamut_sim/runner.py` (ново) — `ScenarioRunner`: оркестрира сценарио симулације. Убацује предмете на улазним зонама, помера сат симулације, бележи одлуке рутирања. Повезује се на MQTT за посматрање догађаја и верификацију исхода.

**Датотеке за креирање/измену**:
- `sim/mamut_sim/layout.py` (заменити stub)
- `sim/mamut_sim/item.py` (ново)
- `sim/mamut_sim/runner.py` (ново)

**Верификација**: `pytest sim/` — учитавање test-bench распореда, убацивање предмета, посматрање проласка кроз 2 зоне са коректним догађајима сензора.

### 6Б — Дефиниције сценарија и валидација (средње)

**Код**:
- `sim/scenarios/basic-sort.yaml` (ново) — један предмет: скенирање → одлука о сортирању → активација скретнице → предмет стиже на коректан излаз
- `sim/scenarios/multi-item.yaml` (ново) — више предмета са различитим одредиштима, тестира конкурентно рутирање
- `sim/scenarios/fault-recovery.yaml` (ново) — убацивање грешке скретнице усред руте, верификација да је аларм подигнут, предмет преусмерен или задржан
- `sim/scenarios/e-stop.yaml` (ново) — хитно заустављање током рада, све зоне стају, предмети задржани на месту
- `sim/mamut_sim/validator.py` (ново) — `ScenarioValidator`: проверава очекиване исходе — предмет стигао на коректно одредиште, аларми покренути/угашени, тајминг у границама

**Датотеке за креирање/измену**:
- `sim/scenarios/` директоријум (ново) са 4 YAML датотеке сценарија
- `sim/mamut_sim/validator.py` (ново)

**Верификација**: `pytest sim/ -k scenario` — сва 4 сценарија пролазе против покренутих сервиса + симулатора.

### 6В — Docker стек за симулацију (мало)

Комплетно Docker Compose окружење које покреће цео POC без иједног хардвера:

- `docker-compose.sim.yml` — комплетан стек:
  - `mosquitto` — MQTT брокер
  - `mamut-gateway` — повезује се на mosquitto
  - `mamut-controller` — оркестрација
  - `mamut-wms-bridge` — mock одлуке о сортирању
  - `mamut-sim` — Python покретач симулатора са test-bench распоредом
  - `mamut-dashboard` — web UI
- Ажурирати `docker/Dockerfile.sim` за инсталацију paho-mqtt, покретање launcher-а

**Верификација**: `docker compose -f docker-compose.sim.yml up` — комплетан систем ради, предмети теку кроз симулирано постројење, контролна табла приказује статус уживо.

---

## Фаза 7: Контролна табла

**Циљ**: Оперативна контролна табла за мониторинг са подацима у реалном времену из симулираног система.

### 7А — Основа и типови (средње)

**Код**:
- `dashboard/src/types/` — TypeScript типови који одговарају proto дефиницијама (генерисани или ручно написани)
- `dashboard/src/api/grpcClient.ts` — подешавање gRPC-web клијента (користећи `@connectrpc/connect-web` или `grpc-web`)
- `dashboard/src/api/services.ts` — типизиране API функције за сваки gRPC сервис
- `dashboard/src/store/store.ts` — Zustand или React context складиште за стање система
- `dashboard/src/hooks/useStreaming.ts` — hook за gRPC стриминг претплате

**Датотеке**: Креирати датотеке у api/, types/, store/, hooks/ директоријумима (замењујући .gitkeep)

### 7Б — Странице и компоненте (велико)

**Документација**: Написати `docs/en/subsystems/conveyor.md`, `docs/en/subsystems/diverter.md`, `docs/sr/subsystems/conveyor.sr.md`, `docs/sr/subsystems/diverter.sr.md`.

**Код**:
- `dashboard/src/pages/OverviewPage.tsx` — преглед система са мрежом статуса зона/скретница
- `dashboard/src/pages/DiverterPage.tsx` — детаљни приказ скретнице са контролама и самотестирањем
- `dashboard/src/pages/AlarmsPage.tsx` — табела активних аларма са дугметом за потврду
- `dashboard/src/pages/TelemetryPage.tsx` — приказ здравственог статуса и метрика
- `dashboard/src/components/ZoneCard.tsx` — картица статуса зоне (стање, брзина, присутност предмета)
- `dashboard/src/components/DiverterCard.tsx` — картица статуса скретнице (стање, смер)
- `dashboard/src/components/AlarmBanner.tsx` — банер обавештења о критичном аларму
- `dashboard/src/components/Layout.tsx` — љуска апликације са навигационом бочном траком
- `dashboard/src/App.tsx` — React Router подешавање са рутирањем страница

**i18n**: Проширити `public/locales/en.json` и `public/locales/sr.json` свим UI стринговима.

**Верификација**: `npm run build` пролази; `npm test` са тестовима компонената; `npm run dev` приказује странице са живим подацима из симулираног стека.

### 7В — Docker и прокси за контролну таблу (мало)

- Имплементирати `docker/Dockerfile.dashboard` — nginx сервира изграђене ресурсе са grpc-web прокси конфигурацијом
- Додати Envoy или nginx gRPC-web прокси конфигурацију у `docker-compose.yml`

**Верификација**: `docker compose up dashboard` сервира UI и проксира gRPC-web до контролера.

---

## Фаза 8: Интеграција, полирање и спремност за хардвер

**Циљ**: End-to-end верификација симулираног стека, комплетна документација и припрема архитектуре за замену хардвером.

### 8А — Комплетност конфигурације (мало)

- Проширити све `config/defaults/*.toml` са пуним документованим шемама
- Допунити `config/sites/belgrade-posta/site.toml` као референтно постављање (користи postal-center распоред)
- Допунити `config/sites/nis-carina/site.toml` као варијанту за царину (користи customs распоред)
- Документовати слојевитост конфигурације у `docs/en/operations/deployment.md`

### 8Б — Интеграциони тестови (средње)

- `services/tests/integration/` — Rust интеграциони тестови: покренути контролер + gateway ин-процесно са уграђеним MQTT брокером (или test контејнером), тестирати комплетан ток скенирање→сортирање→скретање преко gRPC клијената против симулатора
- `dashboard/e2e/` — Playwright тестови против покренутог симулираног backend-а (основни smoke: страница се учитава, зоне приказане, предмет се рутира)
- Проширити `spec/test-vectors/` свеобухватним крос-слојним векторима:
  - `conveyor-zones.json` — вектори прелаза стања зоне
  - `safety-interlocks.json` — вектори валидације блокада
  - `diverter-mdr.json`, `diverter-popup.json`, `diverter-shoe.json`

### 8В — Финализација документације (средње)

Комплетирати све преостале doc stub-ове:
- `docs/en/api/grpc.md` — ауто-генерисана референца сервиса
- `docs/en/architecture/overview.md` — свеобухватни водич за архитектуру, са нагласком на приступ прво-симулација
- `docs/en/operations/deployment.md` — комплетан водич за постављање (Docker сим стек, конфигурација, подешавање сајта)
- Сви `docs/sr/` српски преводи
- `spec/protocols/scada-interface.md` — напомене о SCADA интеграцији

### 8Г — Чеклиста спремности за хардвер (мало)

Документовати пут од POC-а до хардвера:
- `docs/en/operations/hardware-migration.md` (ново) — водич корак по корак:
  1. Који Python симулатор се мапира на који модул фирмвера
  2. Чеклиста усклађености са MQTT уговором о темама
  3. Како покренути мешовити режим (неки стварни уређаји, неки симулирани) за инкременталну миграцију
  4. Водич за HAL имплементацију за нове циљне плоче (референца на постојећи `hal_posix.c`)
  5. Стратегија тестирања фирмвера: поновити исте тест векторе и сценарије против стварног хардвера

### 8Д — Учвршћивање CI/CD (мало)

- Верификовати `.github/workflows/ci.yml` покреће све тестове end-to-end (укључујући сим сценарије)
- Додати корак генерисања proto-а у CI
- Додати корак валидације Docker изградње
- Додати `make sim-test` циљ који покреће Docker сим стек, извршава сценарије, руши стек

**Завршна верификација**: `make all && make test` — Rust сервиси, Python сим и контролна табла се граде и пролазе тестове. `docker compose -f docker-compose.sim.yml up` — комплетно симулирано постројење ради. Сценарији се извршавају end-to-end. Контролна табла приказује живо рутирање предмета.

---

## Граф зависности фаза

```
Фаза 1 (Proto + MQTT уговор + Транспорт)
    ├── Фаза 2 (Телеметрија + Аларми + Спецификације)
    │       └── Фаза 3 (Доменски crate-ови)
    │               └── Фаза 4 (Сервисне бинарне датотеке)
    │                       ├── Фаза 7 (Контролна табла)
    │                       └── Фаза 8 (Интеграција)
    └── Фаза 5 (Симулатори уређаја) — паралелно са фазама 2-4
            └── Фаза 6 (Системска симулација) — потребне Фаза 4 + Фаза 5
```

**Кључна разлика у односу на приступ прво-хардвер**: Фаза 5 (симулатори) може да почне чим се дефинише MQTT уговор (Фаза 1Б). Развој симулатора тече паралелно са развојем сервиса. До Фазе 6 комплетан стек је тестабилан без иједног хардвера.

## Стратегија верификације

Свака фаза има локалну верификацију (јединични тестови, провере компајлирања). Тачке крос-фазне верификације:

| Контролна тачка | Тест |
|-----------------|------|
| После фазе 1 | `cargo build` — сви crate-ови се компајлирају са генерисаним proto типовима |
| После фазе 4 | `cargo test` — сви тестови сервиса пролазе; бинарне датотеке се покрећу |
| После фазе 5 | `pytest sim/` — сви симулатори уређаја објављују коректне MQTT поруке |
| После фазе 6 | `pytest sim/ -k scenario` — комплетни сценарији рутирања предмета пролазе end-to-end |
| После фазе 7 | `npm run build && npm test` — контролна табла се гради и тестови компонената пролазе |
| После фазе 8 | `docker compose -f docker-compose.sim.yml up && make sim-test` — комплетно симулирано постројење ради, сви сценарији зелени |

## Будућност: Интеграција хардвера (није у обиму POC-а)

Када стигне хардвер, пут миграције је:
1. Флешовати C фирмвер који имплементира исти MQTT уговор (`spec/protocols/mqtt-contract.md`)
2. Покренути мешовити режим: стварни уређаји + симулатори на истом MQTT брокеру
3. Замењивати симулаторе један по један уређај, верификујући сваки истим тест сценаријима
4. Core Rust сервиси и контролна табла захтевају **нула промена** — они виде само MQTT поруке
