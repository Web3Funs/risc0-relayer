#[macro_export]
macro_rules! ok_or_continue{
    ($exec: expr, $content: expr)=>{
        match $exec {
            Ok(value) => value,
            Err(e) => {
                error!("{} failed: {}", $content, e);
                continue;
            }
        }
    };
    ($exec: expr, $content: expr, $operation: expr)=>{
      match $exec {
          Ok(value) => value,
          Err(e) => {
              error!("{} failed: {}", $content, e);
              $operation;
          }
      }
    };
}

#[macro_export]
macro_rules! server_handler {
    ($func_name: ident, $input_type: ty, $handler: ident) => {
        async fn $func_name(input: $input_type) -> Result<HttpResponse, Error> {
            match $handler(input.value()) {
                Ok(r) =>{
                    if r == "Buffer overflow."{
                        return Err(error::ErrorInternalServerError("Buffer overflow."));
                    }
                    Ok(HttpResponse::Ok()
                        .json(ResponseMessage{
                            code: 200,
                            msg: r
                        }))
                }
                Err(e) =>{
                    error!("report Faild:{}", e);
                    Err(error::ErrorInternalServerError("report Faild"))
                }
            }
        }
    };

    ($func_name: ident, $handler: ident) => {
        async fn $func_name() -> Result<HttpResponse, Error> {
            match $handler() {
                Ok(r) =>{
                    if r == "Buffer overflow."{
                        return Err(error::ErrorInternalServerError("Buffer overflow."));
                    }
                    Ok(HttpResponse::Ok()
                        .json(ResponseMessage{
                            code: 200,
                            msg: r
                        }))
                }
                Err(e) =>{
                    error!("report Faild:{}", e);
                    Err(error::ErrorInternalServerError("report Faild"))
                }
            }
        }
    };
}
