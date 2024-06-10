use std::time::Duration;

use futures_util::SinkExt;
use smarthome_math::Hsv;
use snake_logic::{get_next_point, Point};
use tokio::time::sleep;
use tokio_tungstenite::connect_async;

const WEBSOCKET: &str = "wss://ledmatrix.edjopato.de/ws";
const WIDTH: u8 = 8;
const HEIGHT: u8 = 32;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    loop {
        if let Err(err) = snake().await {
            eprintln!("pixelmatrix vertical ERROR {err}");
        }
        sleep(Duration::from_secs(10)).await;
    }
}

#[allow(clippy::min_ident_chars)]
#[derive(Debug, serde::Serialize, Clone, Copy)]
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
        Self {
            x,
            y,
            r: 0,
            g: 0,
            b: 0,
        }
    }
}

#[allow(clippy::fallible_impl_from)]
impl From<Pixel> for tokio_tungstenite::tungstenite::Message {
    fn from(val: Pixel) -> Self {
        Self::Text(serde_json::to_string(&val).unwrap())
    }
}

fn hue_to_rgb(hue: u16) -> (u8, u8, u8) {
    Hsv::from_hue(f32::from(hue)).to_rgb_u8()
}

async fn snake() -> anyhow::Result<()> {
    let (mut client, _) = connect_async(WEBSOCKET).await?;

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
            let (red, green, blue) = hue_to_rgb(hue);
            let pixel = Pixel::new(x, y, red, green, blue);
            client.send(pixel.into()).await?;
        }
        snake.insert(0, next_point);

        {
            let Point { x, y } = food;
            let (red, green, blue) = hue_to_rgb((hue + 180) % 360);
            let pixel = Pixel::new(x, y, red, green, blue);
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

    client.flush().await?;
    Ok(())
}
