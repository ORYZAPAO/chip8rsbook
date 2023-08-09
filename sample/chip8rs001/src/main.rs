//
// CHIP-8仮想マシン
//   命令は未実装
//   斜め先を表示するだけ
//

//use std::thread::sleep;
//use std::time::Duration;

extern crate sdl2;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::keyboard::Scancode;
use sdl2::pixels::Color;
use sdl2::render::Canvas;
use sdl2::video::Window;

#[allow(non_snake_case)]
struct Chip8 {
    pub mem: [u8; 0x1000],

    pub reg_V: [u8; 16],     // V0 - V15    Register
    pub reg_I: usize,        // Index       Register
    pub reg_delay_timer: u8, // Delay Timer Register
    pub reg_sound_timer: u8, // Sound Timer Register

    pub pc: usize,        // Program Counter
    pub stack: [u16; 16], // 16byte Stack Area
    pub stack_p: usize,   // Stack Pointer

    pub vram: [[u8; Self::XSIZE]; Self::YSIZE],
}

impl Chip8 {
    pub const CELLSIZE: usize = 12; // 12 x 12ピクセルサイズ
                                    //（ピクセルサイズは適当に変更してください）
    pub const XSIZE: usize = 64; // 横64 ピクセル
    pub const YSIZE: usize = 32; // 縦32 ライン

    pub fn draw(&self, canvas: &mut Canvas<Window>) {
        for yy in 0..Self::YSIZE {
            for xx in 0..Self::XSIZE {
                //
                if self.vram[yy][xx] == 1 {
                    canvas.set_draw_color(Color::RGB(0, 200, 0));
                } else {
                    canvas.set_draw_color(Color::RGB(0, 0, 0));
                }

                //
                let _ = canvas.fill_rect(sdl2::rect::Rect::new(
                    xx as i32 * Self::CELLSIZE as i32,
                    yy as i32 * Self::CELLSIZE as i32,
                    Self::CELLSIZE as u32,
                    Self::CELLSIZE as u32,
                ));
            }
        }
    }
}

fn main() -> Result<(), String> {
    //let mut chip8 : Chip8;
    let mut chip8 = Chip8 {
        mem: [0_u8; 0x1000],

        reg_V: [0_u8; 16],
        reg_I: 0,
        reg_delay_timer: 0,
        reg_sound_timer: 0,

        pc: 0x200, // 実行開始アドレス
        stack: [0_u16; 16],
        stack_p: 0,

        // VRAM領域。オール0 で初期化
        vram: [[0_u8; Chip8::XSIZE]; Chip8::YSIZE],
    };

    // ----------------------------------------
    // SDL2 初期化
    // ----------------------------------------
    let sdl_context = sdl2::init()?;
    let video_system = sdl_context.video()?;

    let window = video_system
        .window(
            "chip8", //
            // ウインドウXサイズ
            (Chip8::CELLSIZE * Chip8::XSIZE) as u32,
            // ウインドウYサイズ
            (Chip8::CELLSIZE * Chip8::YSIZE) as u32,
        )
        .position_centered()
        .opengl()
        .build()
        .map_err(|e| e.to_string())?;

    // グラフィック描画のための canvas を取得
    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;

    // 黒で塗り潰す
    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();
    canvas.present();

    // イベントポンプ
    let mut event_pomp = sdl_context.event_pump()?;

    // ----------------------------------------
    // ----------------------------------------

    // CHIP-8プログラムをメインメモリに準備
    chip8.mem[0x200] = 0x64;
    chip8.mem[0x201] = 0x1A;

    // trueだったら、PCを進めない
    let mut update_pc = false;

    // 経過時間
    const WAIT_MS: i32 = 2; //  2ms Wait
    const CYCLE: i32 = 18; // 18ms cycle
    let mut time_ct = CYCLE; // 18ms毎のダウンカウンタ

    // 実行ループ
    'dec_exec_loop: loop {
        // 現在のキー状態を取得
        let key_state = event_pomp.keyboard_state();

        // キー押下の判定
        if key_state.is_scancode_pressed(Scancode::X) {
            println!("XXXX");
        } else {
            println!();
        }

        // 残りのイベントを処理
        //for event in event_pomp.poll_event() {
        while let Some(event) = event_pomp.poll_event() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'dec_exec_loop,
                _ => {}
            }
        }

        // 命令フェッチ
        //
        // 2byte,16bit値のうち
        // -- 12 〜 15bit目の値
        let d0 = (chip8.mem[chip8.pc] & 0xF0) >> 4;
        // --  8 〜 11bit目
        #[allow(unused_parens)]
        let d1 = (chip8.mem[chip8.pc] & 0x0F);
        // --  4 〜 7bit目
        let d2 = (chip8.mem[chip8.pc + 1] & 0xF0) >> 4;
        // --  0 〜 3bit目(最下位)
        #[allow(unused_parens)]
        let d3 = (chip8.mem[chip8.pc + 1] & 0x0F);

        // デコード、実行
        //
        match (d0, d1, d2, d3) {
            (0x6, x, k1, k0) => {
                // 6xkk - LD Vx, byte
                chip8.reg_V[x as usize] = (k1 << 4) | k0;
            }

            // ******************************
            // ここへ
            // 各CHIP-8命令の実装を追記してゆく
            // ******************************
            _ => {
                // 命令コードが無かったら、直ちに終了
                //break 'dec_exec_loop;
            }
        }

        // プログラムカウンタを+2進める(2byte、16bit分)
        if !update_pc {
            //chip8.pc += 2;
        }

        //
        // 適当にVRAMへ描画（64x32グラフィックのお試し用）
        //
        for i in 0..32 {
            chip8.vram[i][i] = 1;
        }

        // グラフィックを表示
        chip8.draw(&mut canvas);
        canvas.present();

        // 2ms 待つ
        ::std::thread::sleep(::std::time::Duration::from_millis(WAIT_MS as u64));

        // 各タイマーレジスタをカウントダウン
        time_ct -= WAIT_MS;
        if time_ct <= 0 {
            // ディレイタイマーレジスタの更新
            if chip8.reg_delay_timer > 0 {
                chip8.reg_delay_timer -= 1;
            }
            // サウンドタイマーレジスタの更新
            if chip8.reg_sound_timer > 0 {
                chip8.reg_sound_timer -= 1;
            }

            // 初期値に戻す
            time_ct = CYCLE;
        }
    }

    Ok(())
}
