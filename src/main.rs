use std::time::Duration;

use futures_util::SinkExt as _;
use smarthome_math::Hsv;
use snake_logic::{Point, get_next_point};
use tokio::net::TcpStream;
use tokio::time::sleep;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

const WEBSOCKET: &str = "wss://ledmatrix.edjopato.de/ws";
const WIDTH: u8 = 8;
const HEIGHT: u8 = 32;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    loop {
        if let Err(err) = connection().await {
            eprintln!("ERROR {err:#}");
        }
        sleep(Duration::from_secs(30)).await;
    }
}

async fn connection() -> anyhow::Result<()> {
    let (mut client, _) = connect_async(WEBSOCKET).await?;
    loop {
        snake(&mut client).await?;
    }
}

#[expect(clippy::min_ident_chars, reason = "used in websocket protocol")]
#[derive(Debug, serde::Serialize, Clone, Copy)]
#[must_use]
struct Pixel {
    x: u8,
    y: u8,
    r: u8,
    g: u8,
    b: u8,
}

impl Pixel {
    const fn new(x: u8, y: u8, red: u8, green: u8, blue: u8) -> Self {
        Self {
            x,
            y,
            r: red,
            g: green,
            b: blue,
        }
    }

    const fn new_black(x: u8, y: u8) -> Self {
        Self::new(x, y, 0, 0, 0)
    }

    fn new_hue(x: u8, y: u8, hue: u16) -> Self {
        let (red, green, blue) = Hsv::from_hue(f32::from(hue)).to_rgb_u8();
        Self::new(x, y, red, green, blue)
    }
}

#[expect(clippy::fallible_impl_from)]
impl From<Pixel> for tokio_tungstenite::tungstenite::Message {
    fn from(val: Pixel) -> Self {
        Self::text(serde_json::to_string(&val).unwrap())
    }
}

async fn snake(client: &mut WebSocketStream<MaybeTlsStream<TcpStream>>) -> anyhow::Result<()> {
    let mut food = Point::random(WIDTH, HEIGHT);
    let mut hue = rand::random::<u16>() % 360;

    let mut snake = {
        let start = Point::new(WIDTH / 2, HEIGHT / 2);
        let end = {
            let x = if start.x < food.x {
                start.x - 1
            } else {
                start.x + 1
            };
            Point::new(x, start.y)
        };
        vec![start, end]
    };

    while let Some(next_point) = get_next_point(WIDTH, HEIGHT, &snake, food) {
        if snake.contains(&next_point) {
            // Hit itself
            break;
        }

        // println!(
        //     "snake length {:3} goes to {:3} {:3}  food is at {:3} {:3}",
        //     snake.len(),
        //     next_point.x,
        //     next_point.y,
        //     food.x,
        //     food.y
        // );

        if next_point == food {
            food = Point::random(WIDTH, HEIGHT);
        } else {
            let Point { x, y } = snake.pop().unwrap();
            client.send(Pixel::new_black(x, y).into()).await?;
        }

        hue = (hue + 5) % 360;
        {
            let Point { x, y } = next_point;
            let pixel = Pixel::new_hue(x, y, hue);
            client.send(pixel.into()).await?;
        }
        snake.insert(0, next_point);

        {
            let Point { x, y } = food;
            let pixel = Pixel::new_hue(x, y, (hue + 180) % 360);
            client.send(pixel.into()).await?;
        }

        client.flush().await?;
        sleep(Duration::from_millis(250)).await;
    }

    // println!(
    //     "snake length {:3} died at {:3} {:3}",
    //     snake.len(),
    //     snake.first().unwrap().x,
    //     snake.first().unwrap().y,
    // );

    for Point { x, y } in snake {
        client.send(Pixel::new_black(x, y).into()).await?;
        client.flush().await?;
        sleep(Duration::from_millis(100)).await;
    }

    Ok(())
}
