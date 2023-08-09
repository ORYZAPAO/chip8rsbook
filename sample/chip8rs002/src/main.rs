//use std::thread::sleep;
//use std::time::Duration;

use std::io::{BufReader, Read};
use std::path::Path;
use std::{fs, usize};

extern crate sdl2;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::keyboard::Scancode;
use sdl2::pixels::Color;
use sdl2::render::Canvas;
use sdl2::video::Window;

use rand::prelude::*;

#[allow(non_snake_case)]
struct Chip8 {
    pub mem: [u8; 0x1000],

    pub reg_V: [u8; 16],     // V0 - V15   Register
    pub reg_I: usize,        // Index Register
    pub reg_delay_timer: u8, // Delay Timer Register
    pub reg_sound_timer: u8, // Sound Timer Register

    pub pc: usize,                      // Program Counter
    pub stack: [u16; Chip8::STACKSIZE], // 16byte Stack Area
    pub stack_p: usize,                 // Stack Pointer

    pub vram: [[u8; Self::XSIZE]; Self::YSIZE],
    pub rand255: ThreadRng,

    pub wait_for_key: bool,
}

impl Chip8 {
    pub const CELLSIZE: usize = 12; // 12 x 12ピクセルサイズ
                                    //（ピクセルサイズは適当に変更してください）
    pub const XSIZE: usize = 64; // 横64 ピクセル
    pub const YSIZE: usize = 32; // 縦32 ライン
    pub const F_COLOR: sdl2::pixels::Color = Color::RGB(0, 200, 0); // ピクセルの色(Green)
    pub const B_COLOR: sdl2::pixels::Color = Color::RGB(0, 0, 0); // 背景色(Black)

    pub const STACKSIZE: usize = 16; // スタックサイズ 16個

    /// 64x32グラフィックを描画
    ///
    pub fn draw(&self, canvas: &mut Canvas<Window>) {
        for yy in 0..Self::YSIZE {
            for xx in 0..Self::XSIZE {
                //
                if self.vram[yy][xx] == 1 {
                    canvas.set_draw_color(Self::F_COLOR);
                } else {
                    canvas.set_draw_color(Self::B_COLOR);
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

    /// ROMイメージファイルをインメモリ(mem)に読み込む
    ///
    pub fn read_rom<P: AsRef<Path>>(&mut self, romimg: P) -> Result<(), std::io::Error> {
        let fh = fs::File::open(romimg)?;
        let mut reader = BufReader::new(fh);
        let mut tmpmem: [u8; 0x1000] = [0u8; 0x1000];

        let size = reader.read(&mut tmpmem).unwrap_or(0usize);

        //for p in 0..size {
        //    self.mem[0x200 + p] = tmpmem[p];
        //}
        self.mem[0x200..(size + 0x200)].copy_from_slice(&tmpmem[..size]);

        Ok(())
    }

    /// レジスタ表示
    ///
    pub fn report_reg(&self) {
        // 汎用レジスタ
        println!(
            "[PC:{:04x}] {:02x} {:02x}",
            self.pc,
            self.mem[self.pc],
            self.mem[self.pc + 1]
        );
        for i in 0..16 {
            print!(" V{:X}:{:02x}", i, self.reg_V[i]);
            if i == 7 {
                println!();
            }
        }

        // 特殊レジスタ
        println!();
        println!(
            " I:{:04x}  DT:{:02x}  SP:{:02x}",
            self.reg_I, self.reg_delay_timer, self.stack_p
        );

        // スタック領域
        println!(" stack:{:?}", self.stack);

        // メインメモリ
        //print!("  mem[0b37]:{:?}", self.mem[0x0b37]);
        println!();
    }

    /// エラー表示
    ///
    pub fn error_mes<T: AsRef<str>>(&self, mes: T) {
        println!("---");
        println!("--- {}", mes.as_ref());
        println!("---");
        println!(
            "---   [PC:{:04x}] {:02x}{:02x} {:02x}{:02x}",
            self.pc,
            self.mem[self.pc],
            self.mem[self.pc + 1],
            self.mem[self.pc + 2],
            self.mem[self.pc + 3]
        );
    }
}

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("  Usage: chip8rs002 <CHIP-8 ROM Image>");
        return Ok(());
    }
    let chip8_rom = &args[1];

    // Chip8構造体を初期化
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
        rand255: rand::thread_rng(),
        wait_for_key: false,
    };

    // ROMイメージファイルをメインメモリに読み込む
    if chip8.read_rom(chip8_rom).is_err() {
        //if chip8.read_rom("./IBM_Logo.ch8").is_err() {
        panic!(r#"File Not Found!!"#);
    }

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

    // ウインドウの描画領域を全て黒で塗り潰す
    canvas.set_draw_color(Chip8::B_COLOR);
    canvas.clear();
    canvas.present();

    // イベントポンプ取得
    let mut event_pomp = sdl_context.event_pump()?;

    // ----------------------------------------
    // ----------------------------------------

    // 経過時間
    const WAIT_MS: i32 = 2; //  2ms Wait
    const CYCLE: i32 = 18; // 18ms cycle
    let mut time_ct = CYCLE; // 18ms毎のダウンカウンタ

    // 実行ループ
    //let mut result: bool;
    let mut update_pc: bool;
    'dec_exec_loop: loop {
        //result = true;
        update_pc = false;

        // 現在の各レジスタ、スタック内容を表示
        chip8.report_reg();

        // 現在のキー状態を取得
        let key_state = event_pomp.keyboard_state();

        // キー押下の判定
        let all_key_status = [
            // | 1 | 2 | 3 | C |
            if key_state.is_scancode_pressed(Scancode::Num1) {
                1
            } else {
                0xFF
            },
            if key_state.is_scancode_pressed(Scancode::Num2) {
                2
            } else {
                0xFF
            },
            if key_state.is_scancode_pressed(Scancode::Num3) {
                3
            } else {
                0xFF
            },
            if key_state.is_scancode_pressed(Scancode::Num4) {
                0xC
            } else {
                0xFF
            },
            // | 4 | 5 | 6 | D |
            if key_state.is_scancode_pressed(Scancode::Q) {
                4
            } else {
                0xFF
            },
            if key_state.is_scancode_pressed(Scancode::W) {
                5
            } else {
                0xFF
            },
            if key_state.is_scancode_pressed(Scancode::E) {
                6
            } else {
                0xFF
            },
            if key_state.is_scancode_pressed(Scancode::R) {
                0xD
            } else {
                0xFF
            },
            // | 7 | 8 | 9 | E |
            if key_state.is_scancode_pressed(Scancode::A) {
                7
            } else {
                0xFF
            },
            if key_state.is_scancode_pressed(Scancode::S) {
                8
            } else {
                0xFF
            },
            if key_state.is_scancode_pressed(Scancode::D) {
                9
            } else {
                0xFF
            },
            if key_state.is_scancode_pressed(Scancode::F) {
                0xE
            } else {
                0xFF
            },
            // | A | 0 | B | F |
            if key_state.is_scancode_pressed(Scancode::Z) {
                0xA
            } else {
                0xFF
            },
            if key_state.is_scancode_pressed(Scancode::X) {
                0
            } else {
                0xFF
            },
            if key_state.is_scancode_pressed(Scancode::C) {
                0xB
            } else {
                0xFF
            },
            if key_state.is_scancode_pressed(Scancode::V) {
                0xF
            } else {
                0xFF
            },
        ];

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
            (0x0, 0x0, 0xE, 0x0) => {
                // 00E0 - CLS
                for yy in 0..Chip8::YSIZE {
                    for xx in 0..Chip8::XSIZE {
                        chip8.vram[yy][xx] = 0;
                    }
                }
            }
            (0x0, 0x0, 0xE, 0xE) => {
                // 00EE - RET
                if chip8.stack_p > 0 {
                    chip8.pc = chip8.stack[chip8.stack_p - 1] as usize;
                    update_pc = true;
                    chip8.stack_p -= 1;
                } else {
                    // スタックに空である
                    chip8.error_mes("Stack Empty");
                    //result = false;
                    break 'dec_exec_loop;
                }
            }
            (0x0, n2, n1, n0) => {
                // 0nnn - SYS addr
                chip8.pc = ((n2 as u16) << 8 | (n1 as u16) << 4 | (n0 as u16)) as usize;
                update_pc = true;
            }
            (0x1, n2, n1, n0) => {
                // 1nnn - JP addr
                chip8.pc = ((n2 as u16) << 8 | (n1 as u16) << 4 | (n0 as u16)) as usize;
                update_pc = true;
            }
            (0x2, n2, n1, n0) => {
                // 2nnn - CALL addr
                if chip8.stack_p < (16 - 1) {
                    chip8.stack[chip8.stack_p] = ((chip8.pc + 2) & 0xFFFF) as u16;
                    chip8.pc = ((n2 as u16) << 8 | (n1 as u16) << 4 | (n0 as u16)) as usize;
                    update_pc = true;

                    chip8.stack_p += 1;
                } else {
                    // スタックが一杯で、空きが無い
                    chip8.error_mes("Stack Full");
                    //result = false;
                    break 'dec_exec_loop;
                }
            }
            (0x3, x, k1, k0) => {
                // 3xkk - SE Vx, byte
                if chip8.reg_V[x as usize] == (k1 << 4) | k0 {
                    chip8.pc += 4;
                    update_pc = true;
                }
            }
            (0x4, x, k1, k0) => {
                // 4xkk - SNE Vx, byte
                if chip8.reg_V[x as usize] != (k1 << 4 | k0) {
                    chip8.pc += 4;
                    update_pc = true;
                }
            }
            (0x5, x, y, 0x0) => {
                // 5xy0 - SE Vx, Vy
                if chip8.reg_V[x as usize] == chip8.reg_V[y as usize] {
                    chip8.pc += 4;
                    update_pc = true;
                }
            }
            (0x6, x, k1, k0) => {
                // 6xkk - LD Vx, byte
                chip8.reg_V[x as usize] = (k1 << 4) | k0;
            }
            (0x7, x, k1, k0) => {
                // 7xkk - ADD Vx, byte
                let sum = chip8.reg_V[x as usize] as u16 + ((k1 as u16) << 4 | (k0 as u16) & 0xFF);

                // Carry Check
                chip8.reg_V[0xF] = if sum & 0x100 != 0 { 1 } else { 0 };

                chip8.reg_V[x as usize] = sum as u8;
            }
            (0x8, x, y, 0x0) => {
                // 8xy0 - LD Vx, Vy
                chip8.reg_V[x as usize] = chip8.reg_V[y as usize];
            }
            (0x8, x, y, 0x1) => {
                // 8xy1 - OR Vx, Vy
                chip8.reg_V[x as usize] |= chip8.reg_V[y as usize];
            }
            (0x8, x, y, 0x2) => {
                // 8xy2 - AND Vx, Vy
                chip8.reg_V[x as usize] &= chip8.reg_V[y as usize];
            }
            (0x8, x, y, 0x3) => {
                // 8xy3 - XOR Vx, Vy
                chip8.reg_V[x as usize] ^= chip8.reg_V[y as usize];
            }
            (0x8, x, y, 0x4) => {
                // 8xy4 - ADD Vx, Vy
                let sum = chip8.reg_V[x as usize] as u16 + chip8.reg_V[y as usize] as u16;

                // Carry Check
                chip8.reg_V[0xF] = if sum & 0x100 != 0 { 1 } else { 0 };

                chip8.reg_V[x as usize] = sum as u8;
            }
            (0x8, x, y, 0x5) => {
                // 8xy5 - SUB Vx, Vy
                let vx = chip8.reg_V[x as usize];
                let vy = chip8.reg_V[y as usize];

                chip8.reg_V[0xF] = if vx > vy { 1 } else { 0 };
                chip8.reg_V[x as usize] = vx.wrapping_sub(vy);
            }
            (0x8, x, y, 0x6) => {
                // 8xy6 - SHR Vx {, Vy}
                let vx = chip8.reg_V[x as usize];
                let _ = chip8.reg_V[y as usize];

                // LSB check
                chip8.reg_V[0xF] = if vx & 0x01 != 0x00 { 1 } else { 0 };

                chip8.reg_V[x as usize] >>= 1;
            }
            (0x8, x, y, 0x7) => {
                // 8xy7 - SUBN Vx, Vy
                let vx = chip8.reg_V[x as usize];
                let vy = chip8.reg_V[y as usize];

                if vy > vx {
                    chip8.reg_V[0xF] = 1;
                    chip8.reg_V[x as usize] = vy.wrapping_sub(vx);
                } else {
                    chip8.reg_V[0xF] = 0;
                };
            }
            (0x8, x, y, 0xE) => {
                // 8xyE - SHL Vx {, Vy}
                let vx = chip8.reg_V[x as usize] as u16;
                let _ = chip8.reg_V[y as usize] as u16;

                chip8.reg_V[0xF] = if (vx & 0x80) != 0 { 1 } else { 0 };
                chip8.reg_V[x as usize] <<= 1;
            }
            (0x9, x, y, 0x0) => {
                // 9xy0 - SNE Vx, Vy
                if chip8.reg_V[x as usize] != chip8.reg_V[y as usize] {
                    chip8.pc += 4;
                    update_pc = true;
                }
            }
            (0xA, n2, n1, n0) => {
                // Annn - LD I, addr
                chip8.reg_I = ((n2 as u16) << 8 | (n1 as u16) << 4 | (n0 as u16)) as usize;
            }
            (0xB, n2, n1, n0) => {
                // Bnnn - JP V0, addr
                chip8.pc = ((n2 as u16) << 8 | (n1 as u16) << 4 | (n0 as u16)) as usize
                    + chip8.reg_V[0] as usize;
                update_pc = true;
            }
            (0xC, x, k1, k0) => {
                // Cxkk - RND Vx, byte
                let kk = (k1 << 4) | k0;
                chip8.reg_V[x as usize] = chip8.rand255.gen::<u8>() & kk;
            }
            (0xD, x, y, n) => {
                // Dxyn - DRW Vx, Vy, nibble
                // アドレスIのｎバイトのスプライトを読み出し(VX,VY)位置に描画する
                let xx = chip8.reg_V[x as usize] as usize;
                let yy = chip8.reg_V[y as usize] as usize;
                let addr = chip8.reg_I;

                // Update VRAM
                chip8.reg_V[0xf] = 0;
                for byte in 0..(n as usize) {
                    let val = chip8.mem[addr + byte];
                    for bit in 0..8_usize {
                        let sprite_pixel = (val >> (7 - bit)) & 0x1;
                        let mut vram_pixel =
                            chip8.vram[(yy + byte) % Chip8::YSIZE][(xx + bit) % Chip8::XSIZE];

                        chip8.reg_V[0xF] |= sprite_pixel & vram_pixel;
                        vram_pixel ^= sprite_pixel;

                        chip8.vram[(yy + byte) % Chip8::YSIZE][(xx + bit) % Chip8::XSIZE] =
                            vram_pixel;
                    }
                }
            }
            (0xE, x, 0x9, 0xE) => {
                // Ex9E - SKP Vx
                // "キーが押されているか"チェック
                'keyloop: for key in all_key_status.iter() {
                    if chip8.reg_V[x as usize] == *key {
                        chip8.pc += 4;
                        update_pc = true;
                        break 'keyloop;
                    }
                }
            }
            (0xE, x, 0xA, 0x1) => {
                // ExA1 - SKNP Vx
                // "キーが押されていないか"チェック
                let mut not_det_flg = true;
                'keyloop: for key in all_key_status.iter() {
                    if chip8.reg_V[x as usize] == *key {
                        not_det_flg = false;
                        break 'keyloop;
                    }
                }

                if not_det_flg {
                    chip8.pc += 4;
                    update_pc = true;
                }
            }
            (0xF, x, 0x0, 0x7) => {
                // Fx07 - LD Vx, D
                chip8.reg_V[x as usize] = chip8.reg_delay_timer;
            }
            (0xF, x, 0x0, 0xA) => {
                // Fx0A - LD Vx, K
                // キーが入力されるまで全ての実行をストップする。キーが押されるとその値をVxにセットする。
                chip8.wait_for_key = true;

                'keyloop: for key in all_key_status.iter() {
                    if *key != 0xFF {
                        chip8.reg_V[x as usize] = *key;
                        chip8.wait_for_key = false;
                        break 'keyloop;
                    }
                }
            }
            (0xF, x, 0x1, 0x5) => {
                // Fx15 - LD DT, Vx
                chip8.reg_delay_timer = chip8.reg_V[x as usize];
            }
            (0xF, x, 0x1, 0x8) => {
                // Fx18 - LD ST, Vx
                chip8.reg_sound_timer = chip8.reg_V[x as usize];
            }
            (0xF, x, 0x1, 0xE) => {
                // Fx1E - ADD I, Vx
                chip8.reg_I += (chip8.reg_V[x as usize]) as usize;
            }
            (0xF, x, 0x2, 0x09) => {
                chip8.reg_I = chip8.reg_V[x as usize] as usize * 5;
            }
            (0xF, x, 0x3, 0x3) => {
                // Fx33 - LD B, Vx
                let addr = chip8.reg_I;
                let val = chip8.reg_V[x as usize];
                // 10進表記の百の位、十の位、一の位の値を取る
                chip8.mem[addr] = val / 100;
                chip8.mem[addr + 1] = (val % 100) / 10;
                chip8.mem[addr + 2] = val % 10;
            }
            (0xF, x, 0x5, 0x5) => {
                // Fx55 - LD [I], Vx
                for i in 0..(x + 1) as usize {
                    if i < 16 {
                        chip8.mem[chip8.reg_I + i] = chip8.reg_V[i];
                    } else {
                        break;
                    }
                }
            }
            (0xF, x, 0x6, 0x5) => {
                // Fx65 - LD Vx, [I]
                for i in 0..(x + 1) as usize {
                    if i < 16 {
                        chip8.reg_V[i] = chip8.mem[chip8.reg_I];
                        chip8.reg_I += 1;
                    } else {
                        break;
                    }
                }
            }
            _ => {
                // 命令コードが無かったら、直ちに終了
                chip8.error_mes("Not Support Instruction");
                panic!();
            }
        }

        // プログラムカウンタを+2進める(2byte、16bit分)
        // 下記の場合はプログラムカウンタPCを更新しない
        //   1)wait_for_key: true キー入力待ちで、実行を一時停止中のため
        //   2)update_pc   : true 既に各分岐命令でPCを更新済みなので、ここでは更新しない
        if !chip8.wait_for_key && !update_pc {
            chip8.pc += 2;
        } else {
            println!(" --> wait for any key");
        }

        // 64x32グラフィックを表示
        chip8.draw(&mut canvas);
        canvas.present();

        // 2ms 待つ
        //::std::thread::sleep(::std::time::Duration::from_millis(WAIT_MS as u64));
        ::std::thread::sleep(::std::time::Duration::from_micros(500_u64));

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
