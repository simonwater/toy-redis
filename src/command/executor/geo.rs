use crate::{Command, CommandResponse, Context, Value};
use anyhow::{Result, anyhow};
use bytes::Bytes;
use std::sync::Arc;

pub(super) fn execute_geoadd(cmd: &Command, ctx: &Arc<Context>) -> Result<CommandResponse> {
    let mut args_iter = cmd.args.iter();
    let key = args_iter
        .next()
        .ok_or_else(|| anyhow!("geoadd command missing key!"))?
        .to_bulk_bytes()?;
    let longitude = args_iter
        .next()
        .ok_or_else(|| anyhow!("geoadd command missing longitude argument!"))?
        .to_double()?;
    let latitude = args_iter
        .next()
        .ok_or_else(|| anyhow!("geoadd command missing latitude argument!"))?
        .to_double()?;
    let place = args_iter
        .next()
        .ok_or_else(|| anyhow!("geoadd command missing place argument!"))?
        .to_bulk_bytes()?;

    let db = ctx.db_ref();
    let cnt = db.geoadd(key, longitude, latitude, place)?;
    Ok(cnt.into())
}

pub(super) fn execute_geopos(cmd: &Command, ctx: &Arc<Context>) -> Result<CommandResponse> {
    let mut args_iter = cmd.args.iter();
    let key = args_iter
        .next()
        .ok_or_else(|| anyhow!("geopos command missing key!"))?
        .to_bulk_bytes()?;
    let places = args_iter
        .map(|val| val.to_bulk_bytes())
        .collect::<Result<Vec<Bytes>>>()?;

    let db = ctx.db_ref();
    let coords = db
        .geopos(key, places)?
        .into_iter()
        .map(|opt| match opt {
            Some(coord) => coord.into(),
            None => Value::NullArrays,
        })
        .collect::<Vec<Value>>();
    Ok(coords.into())
}

pub(super) fn execute_geodist(cmd: &Command, ctx: &Arc<Context>) -> Result<CommandResponse> {
    let mut args_iter = cmd.args.iter();
    let key = args_iter
        .next()
        .ok_or_else(|| anyhow!("geodist command missing key!"))?
        .to_bulk_bytes()?;
    let place1 = args_iter
        .next()
        .ok_or_else(|| anyhow!("geodist command missing place argument!"))?
        .to_bulk_bytes()?;
    let place2 = args_iter
        .next()
        .ok_or_else(|| anyhow!("geodist command neets two place arguments!"))?
        .to_bulk_bytes()?;

    let db = ctx.db_ref();
    let dist = db.geodist(key, place1, place2)?;
    let val: Value = Bytes::from(dist.to_string()).into();
    Ok(val.into())
}

pub(super) fn execute_geosearch(cmd: &Command, ctx: &Arc<Context>) -> Result<CommandResponse> {
    let mut args_iter = cmd.args.iter();
    let key = args_iter
        .next()
        .ok_or_else(|| anyhow!("geosearch command missing key!"))?
        .to_bulk_bytes()?;

    // center point coordinate mode option: FROMLONLAT
    let _from_xxx = args_iter
        .next()
        .ok_or_else(|| anyhow!("geosearch command missing search mode argument!"))?
        .to_bulk_bytes()?;
    let longitude = args_iter
        .next()
        .ok_or_else(|| anyhow!("geosearch command missing longitude argument!"))?
        .to_double()?;
    let latitude = args_iter
        .next()
        .ok_or_else(|| anyhow!("geosearch command missing latitude argument!"))?
        .to_double()?;

    // searching circular area mode option: BYRADIUS
    let _by_xxx = args_iter
        .next()
        .ok_or_else(|| anyhow!("geosearch command missing byradius argument!"))?
        .to_bulk_bytes()?;
    let radius = args_iter
        .next()
        .ok_or_else(|| anyhow!("geosearch command neets radius arguments!"))?
        .to_double()?;

    let db = ctx.db_ref();
    let targets = db.geosearch(key, longitude, latitude, radius)?;
    Ok(targets.into())
}
