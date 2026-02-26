# Uputstvo za deployment

Ovaj dokument pokriva Fazu 2A: telemetry, health i metrics sloj.

## Preduslovi

- Instaliran Rust toolchain (`cargo`, `rustc`)
- Dostupan MQTT broker (za full-stack pokretanje)
- Slobodni portovi:
  - gRPC servisi: `50051`, `50052`
  - Prometheus scrape endpoint: `9090`

## Telemetry konfiguracija

Podrazumevani fajl: `config/defaults/telemetry.toml`

Ključna polja:
- `log_level`: nivo logovanja (`info`, `debug`, `warn`, ...)
- `prometheus_listen`: endpoint za metrics exporter
- `service_name`: podrazumevani naziv servisa za health odgovor
- `health_default`: inicijalno health stanje
- `metrics_channel_capacity`: velicina in-process stream bafera
- `metrics_flush_interval_ms`: ciljani interval objave metrika

## Build i test

Pokretanje telemetry testova:

```bash
cd services
cargo test -p mamut-telemetry
```

Pokretanje telemetry lint provere:

```bash
cd services
cargo clippy -p mamut-telemetry --all-targets -- -D warnings
```

## Runtime napomene

- `GetHealth` vraca status servisa sa timestamp-om server vremena.
- `StreamMetrics` je server-streaming i podrzava filter po imenu metrike.
- Metrike su Prometheus-kompatibilne i mogu se scrape-ovati sa konfigurisanog endpoint-a.
- Spori stream consumer-i mogu izgubiti starije sample-ove ako se bafer prepuni.

## Observability checklista

1. Potvrdi da health endpoint vraca `healthy=true` za ocekivane servise.
2. Potvrdi da metrics stream emituje sample-ove nakon recording-a.
3. Potvrdi da Prometheus moze da scrape-uje telemetry endpoint.
4. Uvedi alert za prekid telemetry update-a ili kontinuirano lagovanje stream-a.
