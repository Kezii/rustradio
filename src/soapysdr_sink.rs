//! SoapySDR sink.

use anyhow::Result;
use log::debug;

use crate::block::{Block, BlockRet};
use crate::stream::Streamp;
use crate::{Complex, Error};

fn ai_string(ai: &soapysdr::ArgInfo) -> String {
    format!(
        "key={} value={} name={:?} descr={:?} units={:?} data_type={:?} options={:?}",
        ai.key, ai.value, ai.name, ai.description, ai.units, ai.data_type, ai.options
    )
}

/// SoapySDR sink builder.
#[derive(Default)]
pub struct SoapySdrSinkBuilder {
    dev: String,
    channel: usize,
    ogain: f64,
    samp_rate: f64,
    freq: f64,
}

impl SoapySdrSinkBuilder {
    /// Create new builder.
    pub fn new(dev: String, freq: f64, samp_rate: f64) -> Self {
        Self {
            dev,
            freq,
            samp_rate,
            ..Default::default()
        }
    }
    /// Build block.
    pub fn build(self, src: Streamp<Complex>) -> Result<SoapySdrSink> {
        let dev = soapysdr::Device::new(&*self.dev)?;
        debug!("SoapySDR TX driver: {}", dev.driver_key()?);
        debug!("SoapySDR TX hardware: {}", dev.hardware_key()?);
        debug!("SoapySDR TX hardware info: {}", dev.hardware_info()?);
        debug!(
            "SoapySDR TX frontend mapping: {}",
            dev.frontend_mapping(soapysdr::Direction::Tx)?
        );
        let chans = dev.num_channels(soapysdr::Direction::Tx)?;
        debug!("SoapySDR TX channels : {}", chans);
        for channel in 0..chans {
            debug!(
                "SoapySDR TX channel {channel} antennas: {:?}",
                dev.antennas(soapysdr::Direction::Tx, channel)?
            );
            debug!(
                "SoapySDR TX channel {channel} gains: {:?}",
                dev.list_gains(soapysdr::Direction::Tx, channel)?
            );
            debug!(
                "SoapySDR TX channel {channel} frequency range: {:?}",
                dev.frequency_range(soapysdr::Direction::Tx, channel)?
            );
            for ai in dev.stream_args_info(soapysdr::Direction::Tx, channel)? {
                debug!("SoapySDR TX channel {channel} arg info: {}", ai_string(&ai));
            }
            debug!(
                "SoapySDR TX channel {channel} stream formats: {:?}",
                dev.stream_formats(soapysdr::Direction::Tx, channel)?
            );
            debug!(
                "SoapySDR TX channel {channel} info: {}",
                dev.channel_info(soapysdr::Direction::Tx, channel)?
            );
        }
        dev.set_frequency(
            soapysdr::Direction::Tx,
            self.channel,
            self.freq,
            soapysdr::Args::new(),
        )?;
        dev.set_sample_rate(soapysdr::Direction::Tx, self.channel, self.samp_rate)?;
        dev.set_gain(soapysdr::Direction::Tx, self.channel, self.ogain)?;
        let mut stream = dev.tx_stream(&[self.channel])?;
        stream.activate(None)?;
        Ok(SoapySdrSink { src, stream })
    }
}

pub struct SoapySdrSink {
    src: Streamp<Complex>,
    stream: soapysdr::TxStream<Complex>,
}

impl Block for SoapySdrSink {
    fn block_name(&self) -> &str {
        "SoapySdrSink"
    }
    fn work(&mut self) -> Result<BlockRet, Error> {
        let timeout_us = 10_000;
        let (i, _tags) = self.src.read_buf()?;
        if i.len() == 0 {
            return Ok(BlockRet::Noop);
        }
        // debug!("writing {}", i.slice().len());
        let n = match self.stream.write(
            &mut [i.slice()],
            None,  // at_ns
            false, // end_burst
            timeout_us,
        ) {
            Ok(x) => x,
            Err(e) => {
                if e.code == soapysdr::ErrorCode::Timeout {
                    return Ok(BlockRet::Ok);
                }
                return Err(e.into());
            }
        };
        i.consume(n);
        Ok(BlockRet::Noop)
    }
}
