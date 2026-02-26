//! Generated protobuf and gRPC bindings for mamut-diverter services.

/// Namespace for all generated protobuf packages.
pub mod mamut {
    pub mod alarms {
        tonic::include_proto!("mamut.alarms");
    }

    pub mod common {
        tonic::include_proto!("mamut.common");
    }

    pub mod conveyor {
        tonic::include_proto!("mamut.conveyor");
    }

    pub mod diverter {
        tonic::include_proto!("mamut.diverter");
    }

    pub mod identification {
        tonic::include_proto!("mamut.identification");
    }

    pub mod telemetry {
        tonic::include_proto!("mamut.telemetry");
    }

    pub mod tracking {
        tonic::include_proto!("mamut.tracking");
    }

    pub mod wms {
        tonic::include_proto!("mamut.wms");
    }
}

pub use mamut::alarms;
pub use mamut::common;
pub use mamut::conveyor;
pub use mamut::diverter;
pub use mamut::identification;
pub use mamut::telemetry;
pub use mamut::tracking;
pub use mamut::wms;
