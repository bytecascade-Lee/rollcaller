# 第三方许可证

本应用包含下列第三方开源组件，各组件的著作权归其作者所有。



## 许可类型汇总

| 许可证 | 组件数 |
| --- | ---: |
| MIT | 396 |
| Apache-2.0 | 24 |
| Unicode-3.0 | 18 |
| BSD-3-Clause | 8 |
| ISC | 7 |
| MPL-2.0 | 5 |
| Zlib | 3 |
| CDLA-Permissive-2.0 | 2 |
| BSD-2-Clause | 1 |
| MIT-0 | 1 |
| PSF-2.0 | 1 |
| bzip2-1.0.6 | 1 |

注：同时适用多个许可证的组件，按其中较宽松的一项计入本表。

## 组件清单

### Rust（451 个） {#rust}

| 序号 | 组件 | 版本 | 许可证 | 版权 |
| --- | --- | --- | --- | --- |
| 1 | adler2 | 2.0.1 | [MIT](#mit) | Copyright (C) Jonas Schievink <jonasschievink@gmail.com> |
| 2 | aes | 0.9.3 | [MIT](#mit) | Copyright (c) 2018-2024 The RustCrypto Project Developers; Copyright (c) 2018 Artyom Pavlov |
| 3 | aho-corasick | 1.1.5 | [MIT](#mit) | Copyright (c) 2015 Andrew Gallant |
| 4 | alloc-no-stdlib | 3.0.0 | [BSD-3-Clause](#bsd-3-clause) | Copyright (c) 2016 Dropbox, Inc. |
| 5 | alloc-stdlib | 0.3.0 | [BSD-3-Clause](#bsd-3-clause) | Daniel Reiter Horn <danielrh@dropbox.com> |
| 6 | allocator-api2 | 0.2.21 | [MIT](#mit) | Zakarum <zaq.dev@icloud.com> |
| 7 | anyhow | 1.0.104 | [MIT](#mit) | David Tolnay <dtolnay@gmail.com> |
| 8 | ascii | 1.1.0 | [MIT](#mit) | Copyright (c) 2017 Thomas Bahn and contributors; Copyright (c) 2014 The Rust Project Developers |
| 9 | async-trait | 0.1.92 | [MIT](#mit) | David Tolnay <dtolnay@gmail.com> |
| 10 | atoi | 2.0.0 | [MIT](#mit) | Copyright (c) 2017 |
| 11 | atoi_simd | 0.18.1 | [MIT](#mit) | Copyright 2022-NOW Dmitry Rodionov; Copyright (c) 2022 Dmitry Rodionov |
| 12 | atomic | 0.6.1 | [MIT](#mit) | Copyright (c) 2016 The Rust Project Developers |
| 13 | atomic-waker | 1.1.2 | [MIT](#mit) | Copyright (c) 2016 Alex Crichton; Copyright (c) 2017 The Tokio Authors |
| 14 | autocfg | 1.5.1 | [MIT](#mit) | Copyright (c) 2018 Josh Stone |
| 15 | aws-lc-rs | 1.18.2 | [ISC](#isc) + [Apache-2.0](#apache-20) | Copyright Amazon.com, Inc. or its affiliates. |
| 16 | aws-lc-sys | 0.46.0 | [ISC](#isc) + [Apache-2.0](#apache-20) + [MIT](#mit) + [BSD-3-Clause](#bsd-3-clause) + [MIT-0](#mit-0) | Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.; Copyright (c) 2014-2024 Google Inc. |
| 17 | base64 | 0.23.1 | [MIT](#mit) | Copyright (c) 2025 Alice Maz, Marshall Pierce |
| 18 | bigdecimal | 0.4.11 | [MIT](#mit) | Copyright 2023 The BigDecimal-rs Contributors; Copyright (c) 2023 The BigDecimal-rs Contributors |
| 19 | bit-set | 0.8.0 | [MIT](#mit) | Copyright (c) 2023 The Rust Project Developers |
| 20 | bit-vec | 0.8.0 | [MIT](#mit) | Copyright (c) 2023 The Rust Project Developers |
| 21 | bitflags | 1.3.2 | [MIT](#mit) | Copyright (c) 2014 The Rust Project Developers |
| 22 | bitflags | 2.13.2 | [MIT](#mit) | Copyright (c) 2014 The Rust Project Developers |
| 23 | block-buffer | 0.10.4 | [MIT](#mit) | Copyright (c) 2018-2019 The RustCrypto Project Developers |
| 24 | block-buffer | 0.12.1 | [MIT](#mit) | Copyright (c) 2018-2025 The RustCrypto Project Developers |
| 25 | brotli | 9.0.0 | [BSD-3-Clause](#bsd-3-clause) + [MIT](#mit) | Copyright (c) 2016 Dropbox, Inc.; Copyright (c) 2009, 2010, 2013-2016 by the Brotli Authors. |
| 26 | brotli-decompressor | 6.0.1 | [MIT](#mit) | Copyright (c) 2016 Dropbox, Inc. |
| 27 | bs58 | 0.5.1 | [MIT](#mit) | Copyright (c) 2016 The roaring-rs developers. |
| 28 | bumpalo | 3.20.3 | [MIT](#mit) | Copyright (c) 2019 Nick Fitzgerald |
| 29 | bytemuck | 1.25.2 | [MIT](#mit) | Copyright (c) 2019 Daniel "Lokathor" Gee. |
| 30 | byteorder | 1.5.0 | [MIT](#mit) | Copyright (c) 2015 Andrew Gallant |
| 31 | bytes | 1.12.1 | [MIT](#mit) | Copyright (c) 2018 Carl Lerche |
| 32 | bzip2 | 0.6.1 | [MIT](#mit) | Copyright (c) 2014-2025 Alex Crichton and Contributors |
| 33 | calamine | 0.36.1 | [MIT](#mit) | Copyright (c) 2016 Johann Tuffe |
| 34 | camino | 1.2.6 | [MIT](#mit) | Without Boats <saoirse@without.boats>, Ashley Williams <ashley666ashley@gmail.com>, Steve Klabnik <steve@steveklabnik.com>, Rain <rain@sunshowers.io> |
| 35 | cargo-platform | 0.1.9 | [MIT](#mit) | （未声明） |
| 36 | cargo_metadata | 0.19.2 | [MIT](#mit) | Oliver Schneider <git-spam-no-reply9815368754983@oli-obk.de> |
| 37 | cargo_toml | 1.0.1 | [MIT](#mit) | © Kornel Lesiński |
| 38 | cc | 1.10.0 | [MIT](#mit) | Copyright (c) 2014 Alex Crichton |
| 39 | cfb | 0.14.0 | [MIT](#mit) | Copyright (c) 2017 Matthew D. Steele |
| 40 | cfg-if | 1.0.5 | [MIT](#mit) | Copyright (c) 2014 Alex Crichton |
| 41 | cfg_aliases | 0.2.2 | [MIT](#mit) | Copyright (c) 2020 Katharos Technology |
| 42 | chacha20 | 0.10.2 | [MIT](#mit) | Copyright (c) 2019-2026 The RustCrypto Project Developers |
| 43 | chrono | 0.4.45 | [MIT](#mit) | Copyright (c) 2014, Kang Seonghoon. |
| 44 | chunked_transfer | 1.5.0 | [MIT](#mit) | Copyright (c) 2015 The tiny-http Contributors; Copyright (c) 2015 The rust-chunked-transfer Contributors |
| 45 | cipher | 0.4.4 | [MIT](#mit) | Copyright (c) 2016-2020 RustCrypto Developers |
| 46 | cipher | 0.5.2 | [MIT](#mit) | Copyright (c) 2016-2025 RustCrypto Developers |
| 47 | cmake | 0.1.58 | [MIT](#mit) | Copyright (c) 2014 Alex Crichton |
| 48 | cmov | 0.5.4 | [MIT](#mit) | Copyright (c) 2022-2026 The RustCrypto Project Developers |
| 49 | codepage | 0.1.3 | [MIT](#mit) | Copyright Mozilla Foundation |
| 50 | const-oid | 0.10.2 | [MIT](#mit) | Copyright (c) 2020-2026 The RustCrypto Project Developers |
| 51 | constant_time_eq | 0.6.1 | [Apache-2.0](#apache-20) | Cesar Eduardo Barros <cesarb@cesarb.eti.br> |
| 52 | cookie | 0.18.2 | [MIT](#mit) | Copyright 2017 Sergio Benitez; Copyright 2014 Alex Chricton |
| 53 | core_detect | 1.0.0 | [MIT](#mit) | Copyright (c) 2017-2020 The Rust Project Developers |
| 54 | cpubits | 0.1.1 | [MIT](#mit) | Copyright (c) 2023-2026 The RustCrypto Project Developers |
| 55 | cpufeatures | 0.2.17 | [MIT](#mit) | Copyright (c) 2020-2025 The RustCrypto Project Developers |
| 56 | cpufeatures | 0.3.1 | [MIT](#mit) | Copyright (c) 2020-2026 The RustCrypto Project Developers |
| 57 | crc32fast | 1.5.3 | [MIT](#mit) | Copyright (c) 2018 Sam Rijs, Alex Crichton and contributors |
| 58 | crossbeam-channel | 0.5.17 | [MIT](#mit) | Copyright (c) 2019 The Crossbeam Project Developers; Copyright (c) 2009 The Go Authors. All rights reserved. |
| 59 | crossbeam-utils | 0.8.23 | [MIT](#mit) | Copyright (c) 2019 The Crossbeam Project Developers |
| 60 | crossfire | 3.1.22 | [Apache-2.0](#apache-20) | Copyright (C) 2023-2025 The Crossfire Project Developers; Copyright (C) 2016-2023 Yunify Inc. |
| 61 | crossfire-utils | 0.1.1 | [Apache-2.0](#apache-20) | （未声明） |
| 62 | crypto-common | 0.1.7 | [MIT](#mit) | Copyright (c) 2021 RustCrypto Developers |
| 63 | crypto-common | 0.2.2 | [MIT](#mit) | Copyright (c) 2021-2026 RustCrypto Developers |
| 64 | cssparser | 0.37.0 | [MPL-2.0](#mpl-20) | Simon Sapin <simon.sapin@exyr.org> |
| 65 | cssparser-macros | 0.7.1 | [MPL-2.0](#mpl-20) | Simon Sapin <simon.sapin@exyr.org> |
| 66 | ct-codecs | 1.1.8 | [MIT](#mit) | Copyright (c) 2020-2026 Frank Denis |
| 67 | ctor | 1.0.13 | [MIT](#mit) | Matt Mastracci <matthew@mastracci.com> |
| 68 | ctutils | 0.4.5 | [MIT](#mit) | Copyright (c) 2025-2026 The RustCrypto Project Developers |
| 69 | dark-std | 0.2.23 | [MIT](#mit) | zhuxiujia@qq.com |
| 70 | darling | 0.24.1 | [MIT](#mit) | Copyright (c) 2017 Ted Driggs |
| 71 | darling_core | 0.24.1 | [MIT](#mit) | Copyright (c) 2017 Ted Driggs |
| 72 | darling_macro | 0.24.1 | [MIT](#mit) | Copyright (c) 2017 Ted Driggs |
| 73 | debug_unsafe | 0.1.4 | [MIT](#mit) | Copyright 2025-NOW Dmitry Rodionov; Copyright (c) 2025 Dmitry Rodionov |
| 74 | deflate64 | 0.1.12 | [MIT](#mit) | Copyright (c) .NET Foundation and Contributors; Copyright (c) anatawa12 2023 |
| 75 | defmt | 1.2.0 | [MIT](#mit) | Copyright (c) Ferrous Systems |
| 76 | defmt-macros | 1.2.0 | [MIT](#mit) | Copyright (c) Ferrous Systems |
| 77 | defmt-parser | 1.1.0 | [MIT](#mit) | Copyright (c) Ferrous Systems |
| 78 | deranged | 0.5.9 | [MIT](#mit) | Copyright 2024 Jacob Pratt et al.; Copyright (c) 2024 Jacob Pratt et al. |
| 79 | derive_more | 2.1.1 | [MIT](#mit) | Copyright (c) 2016 Jelte Fennema |
| 80 | derive_more-impl | 2.1.1 | [MIT](#mit) | Copyright (c) 2016 Jelte Fennema |
| 81 | digest | 0.10.7 | [MIT](#mit) | Copyright (c) 2017 Artyom Pavlov |
| 82 | digest | 0.11.3 | [MIT](#mit) | Copyright (c) 2017-2025 RustCrypto Developers; Copyright (c) 2017 Artyom Pavlov |
| 83 | directories | 6.0.0 | [MIT](#mit) | Copyright (c) 2018 directories-rs contributors |
| 84 | dirs | 7.0.0 | [MIT](#mit) | Copyright (c) 2018-2019 dirs-rs contributors |
| 85 | dirs-sys | 0.5.0 | [MIT](#mit) | Copyright (c) 2018-2019 dirs-rs contributors |
| 86 | displaydoc | 0.2.7 | [MIT](#mit) | Jane Lusby <jlusby@yaah.dev> |
| 87 | dom_query | 0.28.0 | [MIT](#mit) | Copyright (c) 2023 Mykola Humanov |
| 88 | dpi | 0.1.2 | [Apache-2.0](#apache-20) + [MIT](#mit) | Copyright (c) 2018 Jorge Aparicio; Copyright © 2005-2020 Rich Felker, et al. |
| 89 | dtoa | 1.0.11 | [MIT](#mit) | David Tolnay <dtolnay@gmail.com> |
| 90 | dtoa-short | 0.3.5 | [MPL-2.0](#mpl-20) | Xidorn Quan <me@upsuper.org> |
| 91 | dunce | 1.0.5 | [Apache-2.0](#apache-20) | Kornel <kornel@geekhood.net> |
| 92 | dyn-clone | 1.0.20 | [MIT](#mit) | David Tolnay <dtolnay@gmail.com> |
| 93 | ego-tree | 0.11.1 | [ISC](#isc) | Copyright © 2016, June McEnroe <june@causal.agency>; Copyright © 2024-2025, rust-scraper Contributors |
| 94 | either | 1.19.0 | [MIT](#mit) | Copyright (c) 2015 |
| 95 | embed-resource | 3.0.12 | [MIT](#mit) | Copyright (c) 2017 nabijaczleweli |
| 96 | encoding_rs | 0.8.42 | [Apache-2.0](#apache-20) + [MIT](#mit) + [BSD-3-Clause](#bsd-3-clause) | Copyright Mozilla Foundation; Copyright © WHATWG (Apple, Google, Mozilla, Microsoft). |
| 97 | enigo | 0.1.3 | [MIT](#mit) | Copyright (c) 2017 pythoneer |
| 98 | equivalent | 1.0.3 | [MIT](#mit) | Copyright (c) 2016--2023 |
| 99 | erased-serde | 0.4.10 | [MIT](#mit) | David Tolnay <dtolnay@gmail.com> |
| 100 | fast-float2 | 0.2.4 | [MIT](#mit) | Copyright 2021 Ivan Smirnov |
| 101 | fast_pool | 1.0.4 | [Apache-2.0](#apache-20) | ce <zhuxiujia@qq.com> |
| 102 | fastdate | 0.3.36 | [Apache-2.0](#apache-20) | ce <zhuxiujia@qq.com> |
| 103 | fastrand | 2.5.0 | [MIT](#mit) | Stjepan Glavina <stjepang@gmail.com> |
| 104 | fdeflate | 0.3.7 | [MIT](#mit) | The image-rs Developers |
| 105 | find-msvc-tools | 0.1.15 | [MIT](#mit) | Copyright (c) 2014 Alex Crichton |
| 106 | flate2 | 1.1.10 | [MIT](#mit) | Copyright (c) 2014-2026 Alex Crichton |
| 107 | flume | 0.11.1 | [MIT](#mit) | Joshua Barretto <joshua.s.barretto@gmail.com> |
| 108 | fnv | 1.0.7 | [MIT](#mit) | Copyright (c) 2017 Contributors |
| 109 | foldhash | 0.1.5 | [Zlib](#zlib) | Copyright (c) 2024 Orson Peters |
| 110 | foldhash | 0.2.0 | [Zlib](#zlib) | Copyright (c) 2024 Orson Peters |
| 111 | form_urlencoded | 1.2.2 | [MIT](#mit) | Copyright (c) 2013-2016 The rust-url developers |
| 112 | fs_extra | 1.3.0 | [MIT](#mit) | Copyright (c) 2017 Denis Kurilenko |
| 113 | futures | 0.3.34 | [MIT](#mit) | Copyright (c) 2016 Alex Crichton; Copyright (c) 2017 The Tokio Authors |
| 114 | futures-channel | 0.3.34 | [MIT](#mit) | Copyright (c) 2016 Alex Crichton; Copyright (c) 2017 The Tokio Authors |
| 115 | futures-core | 0.3.34 | [MIT](#mit) | Copyright (c) 2016 Alex Crichton; Copyright (c) 2017 The Tokio Authors |
| 116 | futures-executor | 0.3.34 | [MIT](#mit) | Copyright (c) 2016 Alex Crichton; Copyright (c) 2017 The Tokio Authors |
| 117 | futures-intrusive | 0.5.0 | [MIT](#mit) | Copyright (c) 2019 Matthias Einwag |
| 118 | futures-io | 0.3.34 | [MIT](#mit) | Copyright (c) 2016 Alex Crichton; Copyright (c) 2017 The Tokio Authors |
| 119 | futures-macro | 0.3.34 | [MIT](#mit) | Copyright (c) 2016 Alex Crichton; Copyright (c) 2017 The Tokio Authors |
| 120 | futures-sink | 0.3.34 | [MIT](#mit) | Copyright (c) 2016 Alex Crichton; Copyright (c) 2017 The Tokio Authors |
| 121 | futures-task | 0.3.34 | [MIT](#mit) | Copyright (c) 2016 Alex Crichton; Copyright (c) 2017 The Tokio Authors |
| 122 | futures-util | 0.3.34 | [MIT](#mit) | Copyright (c) 2016 Alex Crichton; Copyright (c) 2017 The Tokio Authors |
| 123 | generic-array | 0.14.7 | [MIT](#mit) | Copyright (c) 2015 Bartłomiej Kamiński |
| 124 | getopts | 0.2.24 | [MIT](#mit) | Copyright (c) 2014 The Rust Project Developers |
| 125 | getrandom | 0.2.17 | [MIT](#mit) | Copyright (c) 2018-2024 The rust-random Project Developers; Copyright (c) 2014 The Rust Project Developers |
| 126 | getrandom | 0.3.4 | [MIT](#mit) | Copyright (c) 2018-2025 The rust-random Project Developers; Copyright (c) 2014 The Rust Project Developers |
| 127 | getrandom | 0.4.3 | [MIT](#mit) | Copyright (c) 2018-2026 The rust-random Project Developers; Copyright (c) 2014 The Rust Project Developers |
| 128 | glob | 0.3.4 | [MIT](#mit) | Copyright (c) 2014 The Rust Project Developers |
| 129 | h2 | 0.4.20 | [MIT](#mit) | Copyright (c) 2017 h2 authors |
| 130 | hashbrown | 0.12.3 | [MIT](#mit) | Copyright (c) 2016 Amanieu d'Antras |
| 131 | hashbrown | 0.15.5 | [MIT](#mit) | Copyright (c) 2016 Amanieu d'Antras |
| 132 | hashbrown | 0.17.1 | [MIT](#mit) | Copyright (c) 2016 Amanieu d'Antras |
| 133 | heck | 0.5.0 | [MIT](#mit) | Copyright (c) 2015 The Rust Project Developers |
| 134 | hex | 0.4.3 | [MIT](#mit) | Copyright (c) 2013-2014 The Rust Project Developers.; Copyright (c) 2015-2020 The rust-hex Developers |
| 135 | hmac | 0.12.1 | [MIT](#mit) | Copyright (c) 2017 Artyom Pavlov |
| 136 | hmac | 0.13.0 | [MIT](#mit) | Copyright (c) 2017 Artyom Pavlov |
| 137 | html5ever | 0.39.0 | [MIT](#mit) | Copyright (c) 2014 The html5ever Project Developers |
| 138 | http | 1.5.0 | [MIT](#mit) | Copyright 2017 http-rs authors; Copyright (c) 2017 http-rs authors |
| 139 | http-body | 1.1.0 | [MIT](#mit) | Copyright (c) 2019-2026 Sean McArthur & Hyper Contributors |
| 140 | http-body-util | 0.1.5 | [MIT](#mit) | Copyright (c) 2019-2026 Sean McArthur & Hyper Contributors |
| 141 | httparse | 1.10.1 | [MIT](#mit) | Copyright (c) 2015-2025 Sean McArthur |
| 142 | httpdate | 1.0.3 | [MIT](#mit) | Copyright (c) 2016 Pyfisch |
| 143 | hybrid-array | 0.4.15 | [MIT](#mit) | Copyright (c) 2022-2026 The RustCrypto Project Developers |
| 144 | hyper | 1.12.0 | [MIT](#mit) | Copyright (c) 2014-2026 Sean McArthur |
| 145 | hyper-rustls | 0.27.10 | [MIT](#mit) | Copyright (c) 2016, Joseph Birr-Pixton <jpixton@gmail.com>; Copyright (c) 2016 Joseph Birr-Pixton <jpixton@gmail.com> |
| 146 | hyper-util | 0.1.21 | [MIT](#mit) | Copyright (c) 2023-2025 Sean McArthur |
| 147 | ico | 0.6.0 | [MIT](#mit) | Copyright (c) 2018 Matthew D. Steele |
| 148 | icu_collections | 2.4.0 | [Unicode-3.0](#unicode-30) | Copyright © 2020-2024 Unicode, Inc. |
| 149 | icu_locale_core | 2.4.0 | [Unicode-3.0](#unicode-30) | Copyright © 2020-2024 Unicode, Inc. |
| 150 | icu_normalizer | 2.4.0 | [Unicode-3.0](#unicode-30) | Copyright © 2020-2024 Unicode, Inc. |
| 151 | icu_normalizer_data | 2.4.0 | [Unicode-3.0](#unicode-30) | Copyright © 2020-2024 Unicode, Inc. |
| 152 | icu_properties | 2.4.0 | [Unicode-3.0](#unicode-30) | Copyright © 2020-2024 Unicode, Inc. |
| 153 | icu_properties_data | 2.4.0 | [Unicode-3.0](#unicode-30) | Copyright © 2020-2024 Unicode, Inc. |
| 154 | icu_provider | 2.4.0 | [Unicode-3.0](#unicode-30) | Copyright © 2020-2024 Unicode, Inc. |
| 155 | ident_case | 1.0.1 | [MIT](#mit) | Ted Driggs <ted.driggs@outlook.com> |
| 156 | idna | 1.1.0 | [MIT](#mit) | Copyright (c) 2013-2025 The rust-url developers |
| 157 | idna_adapter | 1.2.2 | [MIT](#mit) | Copyright (c) The rust-url developers |
| 158 | indexmap | 1.9.3 | [MIT](#mit) | Copyright (c) 2016--2017 |
| 159 | indexmap | 2.14.2 | [MIT](#mit) | Copyright (c) 2016--2017 |
| 160 | infer | 0.22.0 | [MIT](#mit) | Copyright (c) 2019 Bojan |
| 161 | inout | 0.1.4 | [MIT](#mit) | Copyright (c) 2022 The RustCrypto Project Developers; Copyright (c) 2022 Artyom Pavlov |
| 162 | inout | 0.2.5 | [MIT](#mit) | Copyright (c) 2022-2025 The RustCrypto Project Developers; Copyright (c) 2022 Artyom Pavlov |
| 163 | ipnet | 2.12.2 | [MIT](#mit) | Copyright 2017 Juniper Networks, Inc. |
| 164 | itertools | 0.15.0 | [MIT](#mit) | Copyright (c) 2015 |
| 165 | itoa | 1.0.18 | [MIT](#mit) | David Tolnay <dtolnay@gmail.com> |
| 166 | jiff | 0.2.38 | [MIT](#mit) | Copyright (c) 2015 Andrew Gallant |
| 167 | jiff-core | 0.1.1 | [MIT](#mit) | Copyright (c) 2015 Andrew Gallant |
| 168 | jiff-static | 0.2.38 | [MIT](#mit) | Copyright (c) 2015 Andrew Gallant |
| 169 | jiff-tzdb | 0.1.9 | [MIT](#mit) | Copyright (c) 2015 Andrew Gallant |
| 170 | jiff-tzdb-platform | 0.1.3 | [MIT](#mit) | Copyright (c) 2015 Andrew Gallant |
| 171 | jobserver | 0.1.35 | [MIT](#mit) | Copyright (c) 2014 Alex Crichton |
| 172 | json-patch | 4.2.0 | [MIT](#mit) | Copyright (c) 2017 Ivan Dubrov |
| 173 | json5 | 0.4.1 | [ISC](#isc) | Copyright 2018 Callum Oakley |
| 174 | jsonptr | 0.7.1 | [MIT](#mit) | Copyright 2024 Chance Dinkins; Copyright (c) 2022 Chance Dinkins |
| 175 | keyboard-types | 0.8.3 | [MIT](#mit) | Copyright (c) 2017 Pyfisch |
| 176 | lazy_static | 1.5.1 | [MIT](#mit) | Marvin Löbel <loebel.marvin@gmail.com> |
| 177 | libbz2-rs-sys | 0.2.5 | [bzip2-1.0.6](#bzip2-106) | Copyright (C) 1996-2021 Julian R Seward.; Copyright (C) 2019-2020 Federico Mena Quintero |
| 178 | libc | 0.2.191 | [MIT](#mit) | Copyright (c) The Rust Project Developers |
| 179 | libm | 0.2.16 | [MIT](#mit) | Copyright (c) 2018 Jorge Aparicio; Copyright © 2005-2020 Rich Felker, et al. |
| 180 | libsqlite3-sys | 0.36.0 | [MIT](#mit) | Copyright (c) 2014 The rusqlite developers |
| 181 | litemap | 0.8.3 | [Unicode-3.0](#unicode-30) | Copyright © 2020-2024 Unicode, Inc. |
| 182 | lock_api | 0.4.14 | [MIT](#mit) | Copyright (c) 2016 The Rust Project Developers |
| 183 | lock_api | 0.5.0 | [MIT](#mit) | Copyright (c) 2016 The Rust Project Developers |
| 184 | log | 0.4.34 | [MIT](#mit) | Copyright (c) 2014 The Rust Project Developers |
| 185 | lru | 0.14.0 | [MIT](#mit) | Copyright (c) 2016 Jerome Froelich |
| 186 | lru-slab | 0.1.3 | [MIT](#mit) | Copyright (c) 2024 The lru-slab Developers |
| 187 | lzma-rust2 | 0.22.0 | [Apache-2.0](#apache-20) | （未声明） |
| 188 | markup5ever | 0.39.0 | [MIT](#mit) | Copyright (c) 2014 The html5ever Project Developers |
| 189 | matchers | 0.2.0 | [MIT](#mit) | Copyright (c) 2019 Eliza Weisman |
| 190 | memchr | 2.8.3 | [MIT](#mit) | Copyright (c) 2015 Andrew Gallant |
| 191 | mime | 0.3.17 | [MIT](#mit) | Copyright (c) 2014 Sean McArthur |
| 192 | minisign | 0.10.0 | [MIT](#mit) | Copyright (c) 2019-2025 Frank Denis; Copyright (c) 2017 Daniel Rangel |
| 193 | minisign-verify | 0.3.0 | [MIT](#mit) | Copyright (c) 2019-2025 Frank Denis; Copyright (c) 2006-2009 Graydon Hoare |
| 194 | miniz_oxide | 0.8.9 | [MIT](#mit) | Copyright 2013-2014 RAD Game Tools and Valve Software; Copyright 2010-2014 Rich Geldreich and Tenacious Software LLC |
| 195 | miniz_oxide | 0.9.1 | [MIT](#mit) | Copyright 2013-2014 RAD Game Tools and Valve Software; Copyright 2010-2014 Rich Geldreich and Tenacious Software LLC |
| 196 | mio | 1.2.4 | [MIT](#mit) | Copyright (c) 2014 Carl Lerche and other MIO contributors |
| 197 | muda | 0.20.0 | [MIT](#mit) | Copyright (c) 2022-2022 Tauri Programme within The Commons Conservancy |
| 198 | multiversion_no_op | 1.0.0 | [MIT](#mit) | Copyright Mozilla Foundation |
| 199 | new_debug_unreachable | 1.0.6 | [MIT](#mit) | Copyright (c) 2015 Jonathan Reem |
| 200 | nu-ansi-term | 0.50.3 | [MIT](#mit) | Copyright (c) 2014 Benjamin Sago; Copyright (c) 2021-2022 The Nushell Project Developers |
| 201 | num-bigint | 0.4.8 | [MIT](#mit) | Copyright (c) 2014 The Rust Project Developers |
| 202 | num-conv | 0.2.2 | [MIT](#mit) | Copyright (c) Jacob Pratt |
| 203 | num-integer | 0.1.47 | [MIT](#mit) | Copyright (c) 2014 The Rust Project Developers |
| 204 | num-traits | 0.2.19 | [MIT](#mit) | Copyright (c) 2014 The Rust Project Developers |
| 205 | once_cell | 1.21.4 | [MIT](#mit) | Aleksey Kladov <aleksey.kladov@gmail.com> |
| 206 | open | 5.4.5 | [MIT](#mit) | Copyright © `2015` `Sebastian Thiel` |
| 207 | option-ext | 0.2.0 | [MPL-2.0](#mpl-20) | Simon Ochsenreither <simon@ochsenreither.de> |
| 208 | parking_lot | 0.12.5 | [MIT](#mit) | Copyright (c) 2016 The Rust Project Developers |
| 209 | parking_lot | 0.13.0 | [MIT](#mit) | Copyright (c) 2016 The Rust Project Developers |
| 210 | parking_lot_core | 0.10.0 | [MIT](#mit) | Copyright (c) 2016 The Rust Project Developers |
| 211 | parking_lot_core | 0.9.12 | [MIT](#mit) | Copyright (c) 2016 The Rust Project Developers |
| 212 | pbkdf2 | 0.12.2 | [MIT](#mit) | Copyright (c) 2017 Artyom Pavlov; Copyright (c) 2018-2023 The RustCrypto Project Developers |
| 213 | pbkdf2 | 0.13.0 | [MIT](#mit) | Copyright (c) 2017 Artyom Pavlov; Copyright (c) 2018-2023 The RustCrypto Project Developers |
| 214 | percent-encoding | 2.3.2 | [MIT](#mit) | Copyright (c) 2013-2025 The rust-url developers |
| 215 | pest | 2.9.4 | [MIT](#mit) | Dragoș Tiselice <dragostiselice@gmail.com> |
| 216 | pest_derive | 2.9.4 | [MIT](#mit) | Dragoș Tiselice <dragostiselice@gmail.com> |
| 217 | pest_generator | 2.9.4 | [MIT](#mit) | Dragoș Tiselice <dragostiselice@gmail.com> |
| 218 | pest_meta | 2.9.4 | [MIT](#mit) | Dragoș Tiselice <dragostiselice@gmail.com> |
| 219 | phf | 0.13.1 | [MIT](#mit) | Copyright (c) 2014-2022 Steven Fackler, Yuki Okushi |
| 220 | phf_codegen | 0.13.1 | [MIT](#mit) | Copyright (c) 2014-2022 Steven Fackler, Yuki Okushi |
| 221 | phf_generator | 0.13.1 | [MIT](#mit) | Copyright (c) 2014-2022 Steven Fackler, Yuki Okushi |
| 222 | phf_macros | 0.13.1 | [MIT](#mit) | Copyright (c) 2014-2022 Steven Fackler, Yuki Okushi |
| 223 | phf_shared | 0.13.1 | [MIT](#mit) | Copyright (c) 2014-2022 Steven Fackler, Yuki Okushi |
| 224 | pin-project-lite | 0.2.17 | [MIT](#mit) | （未声明） |
| 225 | pkg-config | 0.3.34 | [MIT](#mit) | Copyright (c) 2014 Alex Crichton |
| 226 | plist | 1.10.3 | [MIT](#mit) | Copyright (c) 2015 Edward Barnard |
| 227 | png | 0.18.1 | [MIT](#mit) | Copyright (c) 2015 nwin |
| 228 | pointers | 0.3.1 | [MIT](#mit) | Copyright (c) 2025 naturalio.dev |
| 229 | potential_utf | 0.1.6 | [Unicode-3.0](#unicode-30) | Copyright © 2020-2024 Unicode, Inc. |
| 230 | powerfmt | 0.2.1 | [MIT](#mit) | Copyright 2023 Jacob Pratt et al.; Copyright (c) 2023 Jacob Pratt et al. |
| 231 | ppmd-rust | 1.5.2 | [MIT-0](#mit-0) | Copyright 2026 The ppmd-rust authors |
| 232 | precomputed-hash | 0.1.1 | [MIT](#mit) | Copyright (c) 2017 Emilio Cobos Álvarez |
| 233 | proc-macro2 | 1.0.107 | [MIT](#mit) | David Tolnay <dtolnay@gmail.com>, Alex Crichton <alex@alexcrichton.com> |
| 234 | psm | 0.1.24 | [MIT](#mit) | Copyright © 2018, Simonas Kazlauskas |
| 235 | quick-xml | 0.41.0 | [MIT](#mit) | Copyright (c) 2016 Johann Tuffe |
| 236 | quick-xml | 0.42.0 | [MIT](#mit) | Copyright (c) 2016 Johann Tuffe |
| 237 | quinn | 0.11.12 | [MIT](#mit) | Copyright (c) 2018 The quinn Developers |
| 238 | quinn-proto | 0.11.19 | [MIT](#mit) | Copyright (c) 2018 The quinn Developers |
| 239 | quinn-udp | 0.5.16 | [MIT](#mit) | Copyright (c) 2018 The quinn Developers |
| 240 | quote | 1.0.47 | [MIT](#mit) | David Tolnay <dtolnay@gmail.com> |
| 241 | rand | 0.10.3 | [MIT](#mit) | Copyright 2018 Developers of the Rand project; Copyright (c) 2014 The Rust Project Developers |
| 242 | rand_core | 0.10.1 | [MIT](#mit) | Copyright (c) 2018-2026 The Rand Project Developers |
| 243 | rand_pcg | 0.10.2 | [MIT](#mit) | Copyright (c) 2014-2017 Melissa O'Neill and PCG Project contributors; Copyright 2018 Developers of the Rand project |
| 244 | raw-window-handle | 0.6.2 | [MIT](#mit) | Copyright (c) 2019 Osspial; Copyright (c) 2020 Osspial |
| 245 | rbatis | 4.9.8 | [Apache-2.0](#apache-20) | ce <zhuxiujia@qq.com> |
| 246 | rbatis-codegen | 4.9.3 | [Apache-2.0](#apache-20) | ce <zhuxiujia@qq.com> |
| 247 | rbatis-macro-driver | 4.9.4 | [Apache-2.0](#apache-20) | ce <zhuxiujia@qq.com> |
| 248 | rbdc | 4.9.10 | [Apache-2.0](#apache-20) | ce <zhuxiujia@qq.com> |
| 249 | rbdc-pool-fast | 4.9.5 | [Apache-2.0](#apache-20) | ce <zhuxiujia@qq.com> |
| 250 | rbdc-sqlite | 4.9.5 | [Apache-2.0](#apache-20) | ce <zhuxiujia@qq.com> |
| 251 | rbs | 4.8.4 | [Apache-2.0](#apache-20) | Copyright (c) 2025 rbatis |
| 252 | ref-cast | 1.0.27 | [MIT](#mit) | David Tolnay <dtolnay@gmail.com> |
| 253 | ref-cast-impl | 1.0.27 | [MIT](#mit) | David Tolnay <dtolnay@gmail.com> |
| 254 | regex | 1.13.1 | [MIT](#mit) | Copyright (c) 2014 The Rust Project Developers |
| 255 | regex-automata | 0.4.18 | [MIT](#mit) | Copyright (c) 2014 The Rust Project Developers |
| 256 | regex-syntax | 0.8.11 | [MIT](#mit) | Copyright (c) 2014 The Rust Project Developers |
| 257 | reqwest | 0.13.5 | [MIT](#mit) | Copyright 2016 Sean McArthur; Copyright (c) 2016-2026 Sean McArthur |
| 258 | rfd | 0.16.1 | [MIT](#mit) | Copyright (c) 2022 Bartłomiej Maryńczak |
| 259 | ring | 0.17.14 | [Apache-2.0](#apache-20) + [ISC](#isc) | Copyright (c) 2009 The Go Authors. All rights reserved.; Copyright 2015 The Chromium Authors. All rights reserved. |
| 260 | rolling-file | 0.2.0 | [MIT](#mit) | Copyright (c) 2021 eFolder Inc dba Axcient and contributors.; Copyright (c) 2021, eFolder Inc dba Axcient. |
| 261 | rpassword | 7.5.4 | [Apache-2.0](#apache-20) | Conrad Kleinespel <conradk@conradk.com> |
| 262 | rtoolbox | 0.0.6 | [Apache-2.0](#apache-20) | Conrad Kleinespel <conradk@conradk.com> |
| 263 | rust_xlsxwriter | 0.99.1 | [MIT](#mit) | Copyright 2022-2026 John McNamara <jmcnamara@cpan.org> |
| 264 | rustc-hash | 2.1.3 | [MIT](#mit) | The Rust Project Developers |
| 265 | rustc_version | 0.4.1 | [MIT](#mit) | Copyright (c) 2016 The Rust Project Developers |
| 266 | rustls | 0.23.46 | [MIT](#mit) | Copyright (c) 2016, Joseph Birr-Pixton <jpixton@gmail.com>; Copyright (c) 2016 Joseph Birr-Pixton <jpixton@gmail.com> |
| 267 | rustls-pemfile | 2.2.0 | [MIT](#mit) | Copyright (c) 2016, Joseph Birr-Pixton <jpixton@gmail.com>; Copyright (c) 2016 Joseph Birr-Pixton <jpixton@gmail.com> |
| 268 | rustls-pki-types | 1.15.1 | [MIT](#mit) | Copyright 2023 Dirkjan Ochtman; Copyright (c) 2023 Dirkjan Ochtman <dirkjan@ochtman.nl> |
| 269 | rustls-platform-verifier | 0.7.1 | [MIT](#mit) | Copyright (c) 2022 1Password |
| 270 | rustls-webpki | 0.103.16 | [ISC](#isc) | Copyright 2015 Brian Smith. |
| 271 | rustversion | 1.0.23 | [MIT](#mit) | David Tolnay <dtolnay@gmail.com> |
| 272 | ryu | 1.0.23 | [Apache-2.0](#apache-20) | David Tolnay <dtolnay@gmail.com> |
| 273 | salsa20 | 0.10.2 | [MIT](#mit) | Copyright (c) 2019 Eric McCorkle |
| 274 | same-file | 1.0.6 | [MIT](#mit) | Copyright (c) 2017 Andrew Gallant |
| 275 | schemars | 0.8.22 | [MIT](#mit) | Copyright (c) 2019 Graham Esau |
| 276 | schemars | 0.9.0 | [MIT](#mit) | Copyright (c) 2019 Graham Esau |
| 277 | schemars | 1.2.3 | [MIT](#mit) | Copyright (c) 2019 Graham Esau |
| 278 | schemars_derive | 0.8.22 | [MIT](#mit) | Copyright (c) 2019 Graham Esau |
| 279 | scopeguard | 1.2.0 | [MIT](#mit) | Copyright (c) 2016-2019 Ulrik Sverdrup "bluss" and scopeguard developers |
| 280 | scraper | 0.27.0 | [ISC](#isc) | Copyright © 2016, June McEnroe <june@causal.agency>; Copyright © 2017, Vivek Kushwaha <yoursvivek@gmail.com> |
| 281 | scrypt | 0.11.0 | [MIT](#mit) | Copyright (c) 2021-2023 The RustCrypto Project Developers |
| 282 | selectors | 0.38.0 | [MPL-2.0](#mpl-20) | The Servo Project Developers |
| 283 | semver | 1.0.28 | [MIT](#mit) | David Tolnay <dtolnay@gmail.com> |
| 284 | serde | 1.0.229 | [MIT](#mit) | Erick Tryzelaar <erick.tryzelaar@gmail.com>, David Tolnay <dtolnay@gmail.com> |
| 285 | serde-untagged | 0.1.9 | [MIT](#mit) | David Tolnay <dtolnay@gmail.com> |
| 286 | serde_bytes | 0.11.19 | [MIT](#mit) | David Tolnay <dtolnay@gmail.com> |
| 287 | serde_core | 1.0.229 | [MIT](#mit) | Erick Tryzelaar <erick.tryzelaar@gmail.com>, David Tolnay <dtolnay@gmail.com> |
| 288 | serde_derive | 1.0.229 | [MIT](#mit) | Erick Tryzelaar <erick.tryzelaar@gmail.com>, David Tolnay <dtolnay@gmail.com> |
| 289 | serde_derive_internals | 0.29.1 | [MIT](#mit) | Erick Tryzelaar <erick.tryzelaar@gmail.com>, David Tolnay <dtolnay@gmail.com> |
| 290 | serde_json | 1.0.154 | [MIT](#mit) | （未声明） |
| 291 | serde_repr | 0.1.21 | [MIT](#mit) | David Tolnay <dtolnay@gmail.com> |
| 292 | serde_spanned | 1.1.2 | [MIT](#mit) | Copyright Individual contributors; Copyright (c) Individual contributors |
| 293 | serde_with | 3.24.0 | [MIT](#mit) | Copyright (c) 2015 |
| 294 | serde_with_macros | 3.24.0 | [MIT](#mit) | Copyright (c) 2015 |
| 295 | serde_yaml | 0.9.34+deprecated | [MIT](#mit) | David Tolnay <dtolnay@gmail.com> |
| 296 | serialize-to-javascript | 0.1.2 | [MIT](#mit) | Copyright (c) 2021 Chip Reed |
| 297 | serialize-to-javascript-impl | 0.1.2 | [MIT](#mit) | Copyright (c) 2021 Chip Reed |
| 298 | servo_arc | 0.4.3 | [MIT](#mit) | The Servo Project Developers |
| 299 | sha1 | 0.11.0 | [MIT](#mit) | Copyright (c) 2016-2026 The RustCrypto Project Developers; Copyright (c) 2016 Artyom Pavlov |
| 300 | sha2 | 0.10.9 | [MIT](#mit) | Copyright (c) 2006-2009 Graydon Hoare; Copyright (c) 2009-2013 Mozilla Foundation |
| 301 | sha2 | 0.11.1 | [MIT](#mit) | Copyright (c) 2016-2026 The RustCrypto Project Developers; Copyright (c) 2016 Artyom Pavlov |
| 302 | sharded-slab | 0.1.7 | [MIT](#mit) | Copyright (c) 2019 Eliza Weisman |
| 303 | shlex | 2.0.1 | [MIT](#mit) | Copyright 2015 Nicholas Allegra (comex).; Copyright (c) 2015 Nicholas Allegra (comex). |
| 304 | simd-adler32 | 0.3.10 | [MIT](#mit) | Copyright (c) [2021] [Marvin Countryman] |
| 305 | simdutf8 | 0.1.5 | [MIT](#mit) | Hans Kratz <hans@appfour.com> |
| 306 | siphasher | 1.0.4 | [MIT](#mit) | Copyright 2012-2016 The Rust Project Developers.; Copyright 2016-2026 Frank Denis. |
| 307 | slab | 0.4.12 | [MIT](#mit) | Copyright (c) 2019 Carl Lerche |
| 308 | smallvec | 1.16.4 | [MIT](#mit) | Copyright (c) 2018 The Servo Project Developers |
| 309 | socket2 | 0.6.5 | [MIT](#mit) | Copyright (c) 2014 Alex Crichton |
| 310 | softbuffer | 0.4.8 | [MIT](#mit) | Copyright 2022 Kirill Chibisov |
| 311 | spin | 0.9.9 | [MIT](#mit) | Copyright (c) 2014 Mathijs van de Nes |
| 312 | stable_deref_trait | 1.2.1 | [MIT](#mit) | Copyright (c) 2017 Robert Grosse |
| 313 | stacker | 0.1.25 | [MIT](#mit) | Copyright (c) 2014 Alex Crichton |
| 314 | static_vcruntime | 3.0.0 | [MIT](#mit) | Copyright (c) 2021 Chris Denton. |
| 315 | string_cache | 0.9.0 | [MIT](#mit) | Copyright (c) 2012-2013 Mozilla Foundation |
| 316 | string_cache_codegen | 0.6.1 | [MIT](#mit) | Copyright (c) 2012-2013 Mozilla Foundation |
| 317 | strsim | 0.11.1 | [MIT](#mit) | Copyright (c) 2015 Danny Guo; Copyright (c) 2016 Titus Wormer <tituswormer@gmail.com> |
| 318 | strum | 0.28.0 | [MIT](#mit) | Copyright (c) 2019 Peter Glotfelty |
| 319 | strum_macros | 0.28.0 | [MIT](#mit) | Copyright (c) 2019 Peter Glotfelty |
| 320 | subtle | 2.6.1 | [BSD-3-Clause](#bsd-3-clause) | Copyright (c) 2016-2017 Isis Agora Lovecruft, Henry de Valence. All rights reserved.; Copyright (c) 2016-2024 Isis Agora Lovecruft. All rights reserved. |
| 321 | symlink | 0.1.0 | [MIT](#mit) | Copyright (c) 2014 Chris Morgan and the Teepee project developers |
| 322 | syn | 2.0.119 | [MIT](#mit) | David Tolnay <dtolnay@gmail.com> |
| 323 | syn | 3.0.9 | [MIT](#mit) | David Tolnay <dtolnay@gmail.com> |
| 324 | sync_wrapper | 1.0.2 | [Apache-2.0](#apache-20) | Actyx AG <developer@actyx.io> |
| 325 | synstructure | 0.14.0 | [MIT](#mit) | Copyright 2016 Nika Layzell |
| 326 | tao | 0.37.2 | [Apache-2.0](#apache-20) | Tauri Programme within The Commons Conservancy, The winit contributors |
| 327 | tauri | 2.12.5 | [MIT](#mit) | Copyright (c) 2017 - Present Tauri Apps Contributors |
| 328 | tauri-build | 2.7.4 | [MIT](#mit) | Copyright (c) 2017 - Present Tauri Apps Contributors |
| 329 | tauri-codegen | 2.7.4 | [MIT](#mit) | Copyright (c) 2017 - Present Tauri Apps Contributors |
| 330 | tauri-macros | 2.7.4 | [MIT](#mit) | Copyright (c) 2017 - Present Tauri Apps Contributors |
| 331 | tauri-plugin | 2.7.2 | [MIT](#mit) | Tauri Programme within The Commons Conservancy |
| 332 | tauri-plugin-decorum | 1.1.1 | [MIT](#mit) | Copyright (c) 2024 Siddharth <siddharth99c@gmail.com> |
| 333 | tauri-plugin-dialog | 2.8.3 | [MIT](#mit) | Copyright (c) 2017 - Present Tauri Apps Contributors |
| 334 | tauri-plugin-fs | 2.7.1 | [MIT](#mit) | Copyright (c) 2017 - Present Tauri Apps Contributors |
| 335 | tauri-plugin-opener | 2.7.2 | [MIT](#mit) | Copyright (c) 2017 - Present Tauri Apps Contributors |
| 336 | tauri-plugin-prevent-default | 6.0.1 | [MIT](#mit) | Copyright (c) 2025 Andrew Ferreira <andrew.shien2@gmail.com> |
| 337 | tauri-plugin-single-instance | 2.5.4 | [MIT](#mit) | Copyright (c) 2017 - Present The Tauri Programme in the Commons Conservancy |
| 338 | tauri-plugin-tracing | 0.3.4 | [MIT](#mit) | Flight Science |
| 339 | tauri-runtime | 2.12.2 | [MIT](#mit) | Copyright (c) 2017 - Present Tauri Apps Contributors |
| 340 | tauri-runtime-wry | 2.12.3 | [MIT](#mit) | Copyright (c) 2017 - Present Tauri Apps Contributors |
| 341 | tauri-utils | 2.10.2 | [MIT](#mit) | Copyright (c) 2017 - Present Tauri Apps Contributors |
| 342 | tauri-winres | 0.3.7 | [MIT](#mit) | Copyright (c) 2023 - Present Tauri Apps Contributors; Copyright (c) 2016 Max Resch |
| 343 | tendril | 0.5.1 | [MIT](#mit) | Copyright (c) 2015 Keegan McAllister |
| 344 | termcolor | 1.4.1 | [MIT](#mit) | Copyright (c) 2015 Andrew Gallant |
| 345 | thiserror | 2.0.21 | [MIT](#mit) | David Tolnay <dtolnay@gmail.com> |
| 346 | thiserror-impl | 2.0.21 | [MIT](#mit) | David Tolnay <dtolnay@gmail.com> |
| 347 | thread_local | 1.1.10 | [MIT](#mit) | Copyright (c) 2016 The Rust Project Developers |
| 348 | time | 0.3.55 | [MIT](#mit) | Copyright (c) Jacob Pratt et al. |
| 349 | time-core | 0.1.9 | [MIT](#mit) | Copyright (c) Jacob Pratt et al. |
| 350 | time-macros | 0.2.32 | [MIT](#mit) | Copyright (c) Jacob Pratt et al. |
| 351 | tiny_http | 0.12.0 | [MIT](#mit) | Copyright (c) 2014-2019 The tiny-http contributors |
| 352 | tinystr | 0.8.5 | [Unicode-3.0](#unicode-30) | Copyright © 2020-2024 Unicode, Inc. |
| 353 | tinyvec | 1.13.3 | [MIT](#mit) | Copyright (c) 2019 Daniel "Lokathor" Gee. |
| 354 | tokio | 1.54.1 | [MIT](#mit) | Copyright (c) Tokio Contributors |
| 355 | tokio-macros | 2.7.2 | [MIT](#mit) | Copyright (c) 2019 Yoshua Wuyts; Copyright (c) Tokio Contributors |
| 356 | tokio-rustls | 0.26.6 | [MIT](#mit) | Copyright 2017 quininer kel; Copyright (c) 2017 quininer kel |
| 357 | tokio-util | 0.7.20 | [MIT](#mit) | Copyright (c) Tokio Contributors |
| 358 | toml | 1.1.8+spec-1.1.0 | [MIT](#mit) | Copyright Individual contributors; Copyright (c) Individual contributors |
| 359 | toml_datetime | 1.1.2+spec-1.1.0 | [MIT](#mit) | Copyright Individual contributors; Copyright (c) Individual contributors |
| 360 | toml_parser | 1.1.5+spec-1.1.0 | [MIT](#mit) | Copyright Individual contributors; Copyright (c) Individual contributors |
| 361 | toml_writer | 1.1.3+spec-1.1.0 | [MIT](#mit) | Copyright Individual contributors; Copyright (c) Individual contributors |
| 362 | tower | 0.5.3 | [MIT](#mit) | Copyright (c) 2019 Tower Contributors |
| 363 | tower-http | 0.6.11 | [MIT](#mit) | Copyright (c) 2019-2021 Tower Contributors |
| 364 | tower-layer | 0.3.3 | [MIT](#mit) | Copyright (c) 2019 Tower Contributors |
| 365 | tower-service | 0.3.3 | [MIT](#mit) | Copyright (c) 2019 Tower Contributors |
| 366 | tracing | 0.1.44 | [MIT](#mit) | Copyright (c) 2019 Tokio Contributors |
| 367 | tracing-appender | 0.2.5 | [MIT](#mit) | Copyright (c) 2019 Tokio Contributors |
| 368 | tracing-attributes | 0.1.31 | [MIT](#mit) | Copyright (c) 2019 Tokio Contributors |
| 369 | tracing-core | 0.1.36 | [MIT](#mit) | Copyright (c) 2019 Tokio Contributors |
| 370 | tracing-log | 0.2.0 | [MIT](#mit) | Copyright (c) 2019 Tokio Contributors |
| 371 | tracing-serde | 0.2.0 | [MIT](#mit) | Copyright (c) 2019 Tokio Contributors |
| 372 | tracing-subscriber | 0.3.23 | [MIT](#mit) | Copyright (c) 2019 Tokio Contributors |
| 373 | tray-icon | 0.25.1 | [MIT](#mit) | Copyright (c) 2022-2022 Tauri Programme within The Commons Conservancy |
| 374 | try-lock | 0.2.5 | [MIT](#mit) | Copyright (c) 2018-2023 Sean McArthur; Copyright (c) 2016 Alex Crichton |
| 375 | ts-rs | 12.0.1 | [MIT](#mit) | Moritz Bischof <moritz.bischof1@gmail.com> |
| 376 | ts-rs-macros | 12.0.1 | [MIT](#mit) | Moritz Bischof <moritz.bischof1@gmail.com> |
| 377 | typed-path | 0.12.4 | [MIT](#mit) | Chip Senkbeil <chip@senkbeil.org> |
| 378 | typeid | 1.0.3 | [MIT](#mit) | David Tolnay <dtolnay@gmail.com> |
| 379 | typenum | 1.20.1 | [MIT](#mit) | Copyright 2014 Paho Lurie-Gregg; Copyright (c) 2014 Paho Lurie-Gregg |
| 380 | ucd-trie | 0.1.7 | [MIT](#mit) | Copyright (c) 2015 Andrew Gallant |
| 381 | unicode-ident | 1.0.27 | [MIT](#mit) + [Apache-2.0](#apache-20) + [Unicode-3.0](#unicode-30) | Copyright © 1991-2023 Unicode, Inc. |
| 382 | unicode-segmentation | 1.13.3 | [MIT](#mit) | Copyright (c) 2015 The Rust Project Developers |
| 383 | unicode-width | 0.2.2 | [MIT](#mit) | Copyright (c) 2015 The Rust Project Developers |
| 384 | unsafe-libyaml | 0.2.11 | [MIT](#mit) | David Tolnay <dtolnay@gmail.com> |
| 385 | untrusted | 0.9.0 | [ISC](#isc) | Brian Smith <brian@briansmith.org> |
| 386 | url | 2.5.8 | [MIT](#mit) | Copyright (c) 2013-2025 The rust-url developers |
| 387 | urlpattern | 0.6.0 | [MIT](#mit) | Copyright (c) 2021 the Deno authors |
| 388 | utf8_iter | 1.0.4 | [MIT](#mit) | Copyright Mozilla Foundation |
| 389 | uuid | 1.29.0 | [MIT](#mit) | Copyright (c) 2014 The Rust Project Developers; Copyright (c) 2018 Ashley Mannix, Christopher Armstrong, Dylan DPC, Hunar Roop Kahlon |
| 390 | vcpkg | 0.2.15 | [MIT](#mit) | Copyright (c) 2017 Jim McGrath |
| 391 | version_check | 0.9.5 | [MIT](#mit) | Copyright (c) 2017-2018 Sergio Benitez |
| 392 | vswhom | 0.1.0 | [MIT](#mit) | Copyright (c) 2019 nabijaczleweli |
| 393 | vswhom-sys | 0.1.3 | [MIT](#mit) | Copyright (c) 2019 nabijaczleweli |
| 394 | walkdir | 2.5.0 | [MIT](#mit) | Copyright (c) 2015 Andrew Gallant |
| 395 | want | 0.3.2 | [MIT](#mit) | Copyright (c) 2018-2019 Sean McArthur |
| 396 | web-time | 1.1.0 | [MIT](#mit) | Copyright 2023 dAxpeDDa; Copyright (c) 2023 dAxpeDDa |
| 397 | web_atoms | 0.2.6 | [MIT](#mit) | Copyright (c) 2014 The html5ever Project Developers |
| 398 | webpki-roots | 0.26.11 | [CDLA-Permissive-2.0](#cdla-permissive-20) | （未声明） |
| 399 | webpki-roots | 1.0.11 | [CDLA-Permissive-2.0](#cdla-permissive-20) | （未声明） |
| 400 | webview2-com | 0.39.1 | [MIT](#mit) | （未声明） |
| 401 | webview2-com-macros | 0.8.1 | [MIT](#mit) | （未声明） |
| 402 | webview2-com-sys | 0.39.1 | [MIT](#mit) | （未声明） |
| 403 | winapi-util | 0.1.11 | [MIT](#mit) | Copyright (c) 2017 Andrew Gallant |
| 404 | window-vibrancy | 0.8.1 | [MIT](#mit) | Copyright (c) 2020-2022 Tauri Programme within The Commons Conservancy |
| 405 | windows | 0.51.1 | [MIT](#mit) | Copyright (c) Microsoft Corporation. |
| 406 | windows | 0.62.2 | [MIT](#mit) | Copyright (c) Microsoft Corporation. |
| 407 | windows-collections | 0.3.2 | [MIT](#mit) | Copyright (c) Microsoft Corporation. |
| 408 | windows-core | 0.51.1 | [MIT](#mit) | Copyright (c) Microsoft Corporation. |
| 409 | windows-core | 0.62.2 | [MIT](#mit) | Copyright (c) Microsoft Corporation. |
| 410 | windows-future | 0.3.2 | [MIT](#mit) | Copyright (c) Microsoft Corporation. |
| 411 | windows-implement | 0.60.2 | [MIT](#mit) | Copyright (c) Microsoft Corporation. |
| 412 | windows-interface | 0.59.3 | [MIT](#mit) | Copyright (c) Microsoft Corporation. |
| 413 | windows-link | 0.2.1 | [MIT](#mit) | Copyright (c) Microsoft Corporation. |
| 414 | windows-numerics | 0.3.1 | [MIT](#mit) | Copyright (c) Microsoft Corporation. |
| 415 | windows-registry | 0.6.1 | [MIT](#mit) | Copyright (c) Microsoft Corporation. |
| 416 | windows-result | 0.4.1 | [MIT](#mit) | Copyright (c) Microsoft Corporation. |
| 417 | windows-strings | 0.5.1 | [MIT](#mit) | Copyright (c) Microsoft Corporation. |
| 418 | windows-sys | 0.52.0 | [MIT](#mit) | Copyright (c) Microsoft Corporation. |
| 419 | windows-sys | 0.60.2 | [MIT](#mit) | Copyright (c) Microsoft Corporation. |
| 420 | windows-sys | 0.61.2 | [MIT](#mit) | Copyright (c) Microsoft Corporation. |
| 421 | windows-targets | 0.48.5 | [MIT](#mit) | Copyright (c) Microsoft Corporation. |
| 422 | windows-targets | 0.52.6 | [MIT](#mit) | Copyright (c) Microsoft Corporation. |
| 423 | windows-targets | 0.53.5 | [MIT](#mit) | Copyright (c) Microsoft Corporation. |
| 424 | windows-threading | 0.2.1 | [MIT](#mit) | Copyright (c) Microsoft Corporation. |
| 425 | windows-version | 0.1.7 | [MIT](#mit) | Copyright (c) Microsoft Corporation. |
| 426 | windows_aarch64_msvc | 0.48.5 | [MIT](#mit) | Copyright (c) Microsoft Corporation. |
| 427 | windows_aarch64_msvc | 0.52.6 | [MIT](#mit) | Copyright (c) Microsoft Corporation. |
| 428 | windows_aarch64_msvc | 0.53.1 | [MIT](#mit) | Copyright (c) Microsoft Corporation. |
| 429 | windows_x86_64_msvc | 0.48.5 | [MIT](#mit) | Copyright (c) Microsoft Corporation. |
| 430 | windows_x86_64_msvc | 0.52.6 | [MIT](#mit) | Copyright (c) Microsoft Corporation. |
| 431 | windows_x86_64_msvc | 0.53.1 | [MIT](#mit) | Copyright (c) Microsoft Corporation. |
| 432 | winnow | 1.0.4 | [MIT](#mit) | （未声明） |
| 433 | winreg | 0.56.0 | [MIT](#mit) | Copyright (c) 2015 Igor Shaula |
| 434 | writeable | 0.6.4 | [Unicode-3.0](#unicode-30) | Copyright © 2020-2024 Unicode, Inc. |
| 435 | wry | 0.57.1 | [MIT](#mit) | Copyright (c) 2020-2023 Ngo Iok Ui & Tauri Programme within The Commons Conservancy |
| 436 | yoke | 0.8.4 | [Unicode-3.0](#unicode-30) | Copyright © 2020-2024 Unicode, Inc. |
| 437 | yoke-derive | 0.8.4 | [Unicode-3.0](#unicode-30) | Copyright © 2020-2024 Unicode, Inc. |
| 438 | zerofrom | 0.1.8 | [Unicode-3.0](#unicode-30) | Copyright © 2020-2024 Unicode, Inc. |
| 439 | zerofrom-derive | 0.1.8 | [Unicode-3.0](#unicode-30) | Copyright © 2020-2024 Unicode, Inc. |
| 440 | zeroize | 1.9.1 | [MIT](#mit) | Copyright (c) 2018-2026 The RustCrypto Project Developers |
| 441 | zerotrie | 0.2.5 | [Unicode-3.0](#unicode-30) | Copyright © 2020-2024 Unicode, Inc. |
| 442 | zerovec | 0.11.9 | [Unicode-3.0](#unicode-30) | Copyright © 2020-2024 Unicode, Inc. |
| 443 | zerovec-derive | 0.11.7 | [Unicode-3.0](#unicode-30) | Copyright © 2020-2024 Unicode, Inc. |
| 444 | zip | 8.6.0 | [MIT](#mit) | Copyright (c) 2014 Mathijs van de Nes |
| 445 | zip | 9.0.5 | [MIT](#mit) | Copyright (c) 2014 Mathijs van de Nes |
| 446 | zlib-rs | 0.6.8 | [Zlib](#zlib) | (C) 2024 Trifecta Tech Foundation |
| 447 | zmij | 1.0.23 | [MIT](#mit) | David Tolnay <dtolnay@gmail.com> |
| 448 | zopfli | 0.8.4 | [Apache-2.0](#apache-20) | Copyright 2011 Google Inc. |
| 449 | zstd | 0.14.3 | [BSD-3-Clause](#bsd-3-clause) | Copyright (c) 2026, Alexandre Bury |
| 450 | zstd-safe | 8.2.0 | [BSD-3-Clause](#bsd-3-clause) | Copyright (c) 2026, Alexandre Bury |
| 451 | zstd-sys | 2.1.2+zstd.1.5.7 | [BSD-3-Clause](#bsd-3-clause) | Copyright (c) 2026, Alexandre Bury; Copyright (c) Meta Platforms, Inc. and affiliates. All rights reserved. |

### Node.js（16 个） {#nodejs}

| 序号 | 组件 | 版本 | 许可证 | 版权 |
| --- | --- | --- | --- | --- |
| 1 | @fltsci/tauri-plugin-tracing | 0.3.4 | [MIT](#mit) | Flight Science |
| 2 | @tauri-apps/api | 2.11.0 | [MIT](#mit) | Copyright (c) 2017 - Present Tauri Apps Contributors |
| 3 | @tauri-apps/api | 2.12.4 | [MIT](#mit) | Copyright (c) 2017 - Present Tauri Apps Contributors |
| 4 | @tauri-apps/plugin-dialog | 2.8.3 | [MIT](#mit) | （未声明） |
| 5 | @tauri-apps/plugin-opener | 2.7.2 | [MIT](#mit) | （未声明） |
| 6 | @types/trusted-types | 2.0.7 | [MIT](#mit) | Copyright (c) Microsoft Corporation. |
| 7 | argparse | 3.0.4 | [PSF-2.0](#psf-20) | （未声明） |
| 8 | dompurify | 3.4.17 | [Apache-2.0](#apache-20) | Dr.-Ing. Mario Heiderich, Cure53 |
| 9 | entities | 8.1.0 | [BSD-2-Clause](#bsd-2-clause) | Copyright (c) Felix Böhm |
| 10 | highlight.js | 11.12.0 | [BSD-3-Clause](#bsd-3-clause) | Copyright (c) 2006, Ivan Sagalaev. |
| 11 | linkify-it | 6.1.0 | [MIT](#mit) | Copyright (c) 2015 Vitaly Puzrin. |
| 12 | markdown-it | 15.0.3 | [MIT](#mit) | Copyright (c) 2014 Vitaly Puzrin, Alex Kocharin. |
| 13 | mdurl | 2.1.0 | [MIT](#mit) | Copyright (c) 2015 Vitaly Puzrin, Alex Kocharin.; Copyright Joyent, Inc. and other Node contributors. All rights reserved. |
| 14 | open-props | 1.7.23 | [MIT](#mit) | Copyright (c) 2021 Adam Argyle |
| 15 | punycode.js | 2.3.1 | [MIT](#mit) | Copyright Mathias Bynens <https://mathiasbynens.be/> |
| 16 | uc.micro | 3.0.0 | [MIT](#mit) | Copyright Mathias Bynens <https://mathiasbynens.be/> |

## 许可证内容

同一许可证的正文只列一次；各组件的版权声明见上方清单。

### MIT {#mit}

```text
Permission is hereby granted, free of charge, to any
person obtaining a copy of this software and associated
documentation files (the "Software"), to deal in the
Software without restriction, including without
limitation the rights to use, copy, modify, merge,
publish, distribute, sublicense, and/or sell copies of
the Software, and to permit persons to whom the Software
is furnished to do so, subject to the following
conditions:

The above copyright notice and this permission notice
shall be included in all copies or substantial portions
of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
DEALINGS IN THE SOFTWARE.
```

### Apache-2.0 {#apache-20}

```text
Apache License
                        Version 2.0, January 2004
                     https://www.apache.org/licenses/LICENSE-2.0

TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

1. Definitions.

   "License" shall mean the terms and conditions for use, reproduction,
   and distribution as defined by Sections 1 through 9 of this document.

   "Licensor" shall mean the copyright owner or entity authorized by
   the copyright owner that is granting the License.

   "Legal Entity" shall mean the union of the acting entity and all
   other entities that control, are controlled by, or are under common
   control with that entity. For the purposes of this definition,
   "control" means (i) the power, direct or indirect, to cause the
   direction or management of such entity, whether by contract or
   otherwise, or (ii) ownership of fifty percent (50%) or more of the
   outstanding shares, or (iii) beneficial ownership of such entity.

   "You" (or "Your") shall mean an individual or Legal Entity
   exercising permissions granted by this License.

   "Source" form shall mean the preferred form for making modifications,
   including but not limited to software source code, documentation
   source, and configuration files.

   "Object" form shall mean any form resulting from mechanical
   transformation or translation of a Source form, including but
   not limited to compiled object code, generated documentation,
   and conversions to other media types.

   "Work" shall mean the work of authorship, whether in Source or
   Object form, made available under the License, as indicated by a
   copyright notice that is included in or attached to the work
   (an example is provided in the Appendix below).

   "Derivative Works" shall mean any work, whether in Source or Object
   form, that is based on (or derived from) the Work and for which the
   editorial revisions, annotations, elaborations, or other modifications
   represent, as a whole, an original work of authorship. For the purposes
   of this License, Derivative Works shall not include works that remain
   separable from, or merely link (or bind by name) to the interfaces of,
   the Work and Derivative Works thereof.

   "Contribution" shall mean any work of authorship, including
   the original version of the Work and any modifications or additions
   to that Work or Derivative Works thereof, that is intentionally
   submitted to Licensor for inclusion in the Work by the copyright owner
   or by an individual or Legal Entity authorized to submit on behalf of
   the copyright owner. For the purposes of this definition, "submitted"
   means any form of electronic, verbal, or written communication sent
   to the Licensor or its representatives, including but not limited to
   communication on electronic mailing lists, source code control systems,
   and issue tracking systems that are managed by, or on behalf of, the
   Licensor for the purpose of discussing and improving the Work, but
   excluding communication that is conspicuously marked or otherwise
   designated in writing by the copyright owner as "Not a Contribution."

   "Contributor" shall mean Licensor and any individual or Legal Entity
   on behalf of whom a Contribution has been received by Licensor and
   subsequently incorporated within the Work.

2. Grant of Copyright License. Subject to the terms and conditions of
   this License, each Contributor hereby grants to You a perpetual,
   worldwide, non-exclusive, no-charge, royalty-free, irrevocable
   copyright license to reproduce, prepare Derivative Works of,
   publicly display, publicly perform, sublicense, and distribute the
   Work and such Derivative Works in Source or Object form.

3. Grant of Patent License. Subject to the terms and conditions of
   this License, each Contributor hereby grants to You a perpetual,
   worldwide, non-exclusive, no-charge, royalty-free, irrevocable
   (except as stated in this section) patent license to make, have made,
   use, offer to sell, sell, import, and otherwise transfer the Work,
   where such license applies only to those patent claims licensable
   by such Contributor that are necessarily infringed by their
   Contribution(s) alone or by combination of their Contribution(s)
   with the Work to which such Contribution(s) was submitted. If You
   institute patent litigation against any entity (including a
   cross-claim or counterclaim in a lawsuit) alleging that the Work
   or a Contribution incorporated within the Work constitutes direct
   or contributory patent infringement, then any patent licenses
   granted to You under this License for that Work shall terminate
   as of the date such litigation is filed.

4. Redistribution. You may reproduce and distribute copies of the
   Work or Derivative Works thereof in any medium, with or without
   modifications, and in Source or Object form, provided that You
   meet the following conditions:

   (a) You must give any other recipients of the Work or
       Derivative Works a copy of this License; and

   (b) You must cause any modified files to carry prominent notices
       stating that You changed the files; and

   (c) You must retain, in the Source form of any Derivative Works
       that You distribute, all copyright, patent, trademark, and
       attribution notices from the Source form of the Work,
       excluding those notices that do not pertain to any part of
       the Derivative Works; and

   (d) If the Work includes a "NOTICE" text file as part of its
       distribution, then any Derivative Works that You distribute must
       include a readable copy of the attribution notices contained
       within such NOTICE file, excluding those notices that do not
       pertain to any part of the Derivative Works, in at least one
       of the following places: within a NOTICE text file distributed
       as part of the Derivative Works; within the Source form or
       documentation, if provided along with the Derivative Works; or,
       within a display generated by the Derivative Works, if and
       wherever such third-party notices normally appear. The contents
       of the NOTICE file are for informational purposes only and
       do not modify the License. You may add Your own attribution
       notices within Derivative Works that You distribute, alongside
       or as an addendum to the NOTICE text from the Work, provided
       that such additional attribution notices cannot be construed
       as modifying the License.

   You may add Your own copyright statement to Your modifications and
   may provide additional or different license terms and conditions
   for use, reproduction, or distribution of Your modifications, or
   for any such Derivative Works as a whole, provided Your use,
   reproduction, and distribution of the Work otherwise complies with
   the conditions stated in this License.

5. Submission of Contributions. Unless You explicitly state otherwise,
   any Contribution intentionally submitted for inclusion in the Work
   by You to the Licensor shall be under the terms and conditions of
   this License, without any additional terms or conditions.
   Notwithstanding the above, nothing herein shall supersede or modify
   the terms of any separate license agreement you may have executed
   with Licensor regarding such Contributions.

6. Trademarks. This License does not grant permission to use the trade
   names, trademarks, service marks, or product names of the Licensor,
   except as required for reasonable and customary use in describing the
   origin of the Work and reproducing the content of the NOTICE file.

7. Disclaimer of Warranty. Unless required by applicable law or
   agreed to in writing, Licensor provides the Work (and each
   Contributor provides its Contributions) on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
   implied, including, without limitation, any warranties or conditions
   of TITLE, NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A
   PARTICULAR PURPOSE. You are solely responsible for determining the
   appropriateness of using or redistributing the Work and assume any
   risks associated with Your exercise of permissions under this License.

8. Limitation of Liability. In no event and under no legal theory,
   whether in tort (including negligence), contract, or otherwise,
   unless required by applicable law (such as deliberate and grossly
   negligent acts) or agreed to in writing, shall any Contributor be
   liable to You for damages, including any direct, indirect, special,
   incidental, or consequential damages of any character arising as a
   result of this License or out of the use or inability to use the
   Work (including but not limited to damages for loss of goodwill,
   work stoppage, computer failure or malfunction, or any and all
   other commercial damages or losses), even if such Contributor
   has been advised of the possibility of such damages.

9. Accepting Warranty or Additional Liability. While redistributing
   the Work or Derivative Works thereof, You may choose to offer,
   and charge a fee for, acceptance of support, warranty, indemnity,
   or other liability obligations and/or rights consistent with this
   License. However, in accepting such obligations, You may act only
   on Your own behalf and on Your sole responsibility, not on behalf
   of any other Contributor, and only if You agree to indemnify,
   defend, and hold each Contributor harmless for any liability
   incurred by, or claims asserted against, such Contributor by reason
   of your accepting any such warranty or additional liability.

END OF TERMS AND CONDITIONS

APPENDIX: How to apply the Apache License to your work.

   To apply the Apache License to your work, attach the following
   boilerplate notice, with the fields enclosed by brackets "[]"
   replaced with your own identifying information. (Don't include
   the brackets!)  The text should be enclosed in the appropriate
   comment syntax for the file format. We also recommend that a
   file or class name and description of purpose be included on the
   same "printed page" as the copyright notice for easier
   identification within third-party archives.

Copyright [yyyy] [name of copyright owner]

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

	https://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
```

### BSD-3-Clause {#bsd-3-clause}

```text
All rights reserved.

Redistribution and use in source and binary forms, with or without modification, are permitted provided that the following conditions are met:

1. Redistributions of source code must retain the above copyright notice, this list of conditions and the following disclaimer.

2. Redistributions in binary form must reproduce the above copyright notice, this list of conditions and the following disclaimer in the documentation and/or other materials provided with the distribution.

3. Neither the name of the copyright holder nor the names of its contributors may be used to endorse or promote products derived from this software without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
```

### BSD-2-Clause {#bsd-2-clause}

```text
All rights reserved.

Redistribution and use in source and binary forms, with or without modification, are permitted provided that the following conditions are met:

Redistributions of source code must retain the above copyright notice, this list of conditions and the following disclaimer.

Redistributions in binary form must reproduce the above copyright notice, this list of conditions and the following disclaimer in the documentation and/or other materials provided with the distribution.

THIS IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS,
EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
```

### ISC {#isc}

```text
Permission to use, copy, modify, and/or distribute this software for
any purpose with or without fee is hereby granted.

THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN
AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT
OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
```

### Zlib {#zlib}

```text
This software is provided 'as-is', without any express or implied warranty. In no event will the authors be held liable for any damages arising from the use of this software.

Permission is granted to anyone to use this software for any purpose, including commercial applications, and to alter it and redistribute it freely, subject to the following restrictions:

1. The origin of this software must not be misrepresented; you must not claim that you wrote the original software. If you use this software in a product, an acknowledgment in the product documentation would be appreciated but is not required.

2. Altered source versions must be plainly marked as such, and must not be misrepresented as being the original software.

3. This notice may not be removed or altered from any source distribution.
```

### MIT-0 {#mit-0}

```text
MIT No Attribution

Copyright 2026 The ppmd-rust authors

Permission is hereby granted, free of charge, to any person obtaining a copy of this
software and associated documentation files (the "Software"), to deal in the Software
without restriction, including without limitation the rights to use, copy, modify,
merge, publish, distribute, sublicense, and/or sell copies of the Software, and to
permit persons to whom the Software is furnished to do so.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED,
INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT
HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE
SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
```

### Unicode-3.0 {#unicode-30}

```text
UNICODE LICENSE V3

COPYRIGHT AND PERMISSION NOTICE

Copyright © 2020-2024 Unicode, Inc.

NOTICE TO USER: Carefully read the following legal agreement. BY
DOWNLOADING, INSTALLING, COPYING OR OTHERWISE USING DATA FILES, AND/OR
SOFTWARE, YOU UNEQUIVOCALLY ACCEPT, AND AGREE TO BE BOUND BY, ALL OF THE
TERMS AND CONDITIONS OF THIS AGREEMENT. IF YOU DO NOT AGREE, DO NOT
DOWNLOAD, INSTALL, COPY, DISTRIBUTE OR USE THE DATA FILES OR SOFTWARE.

Permission is hereby granted, free of charge, to any person obtaining a
copy of data files and any associated documentation (the "Data Files") or
software and any associated documentation (the "Software") to deal in the
Data Files or Software without restriction, including without limitation
the rights to use, copy, modify, merge, publish, distribute, and/or sell
copies of the Data Files or Software, and to permit persons to whom the
Data Files or Software are furnished to do so, provided that either (a)
this copyright and permission notice appear with all copies of the Data
Files or Software, or (b) this copyright and permission notice appear in
associated Documentation.

THE DATA FILES AND SOFTWARE ARE PROVIDED "AS IS", WITHOUT WARRANTY OF ANY
KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT OF
THIRD PARTY RIGHTS.

IN NO EVENT SHALL THE COPYRIGHT HOLDER OR HOLDERS INCLUDED IN THIS NOTICE
BE LIABLE FOR ANY CLAIM, OR ANY SPECIAL INDIRECT OR CONSEQUENTIAL DAMAGES,
OR ANY DAMAGES WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS,
WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION,
ARISING OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THE DATA
FILES OR SOFTWARE.

Except as contained in this notice, the name of a copyright holder shall
not be used in advertising or otherwise to promote the sale, use or other
dealings in these Data Files or Software without prior written
authorization of the copyright holder.

SPDX-License-Identifier: Unicode-3.0

—

Portions of ICU4X may have been adapted from ICU4C and/or ICU4J.
ICU 1.8.1 to ICU 57.1 © 1995-2016 International Business Machines Corporation and others.
```

### CDLA-Permissive-2.0 {#cdla-permissive-20}

```text
# Community Data License Agreement - Permissive - Version 2.0

This is the Community Data License Agreement - Permissive, Version
2.0 (the "agreement"). Data Provider(s) and Data Recipient(s) agree
as follows:

## 1. Provision of the Data

1.1. A Data Recipient may use, modify, and share the Data made
available by Data Provider(s) under this agreement if that Data
Recipient follows the terms of this agreement.

1.2. This agreement does not impose any restriction on a Data
Recipient's use, modification, or sharing of any portions of the
Data that are in the public domain or that may be used, modified,
or shared under any other legal exception or limitation.

## 2. Conditions for Sharing Data

2.1. A Data Recipient may share Data, with or without modifications, so
long as the Data Recipient makes available the text of this agreement
with the shared Data.

## 3. No Restrictions on Results

3.1. This agreement does not impose any restriction or obligations
with respect to the use, modification, or sharing of Results.

## 4. No Warranty; Limitation of Liability

4.1. All Data Recipients receive the Data subject to the following
terms:

THE DATA IS PROVIDED ON AN "AS IS" BASIS, WITHOUT REPRESENTATIONS,
WARRANTIES OR CONDITIONS OF ANY KIND, EITHER EXPRESS OR IMPLIED
INCLUDING, WITHOUT LIMITATION, ANY WARRANTIES OR CONDITIONS OF TITLE,
NON-INFRINGEMENT, MERCHANTABILITY OR FITNESS FOR A PARTICULAR PURPOSE.

NO DATA PROVIDER SHALL HAVE ANY LIABILITY FOR ANY DIRECT, INDIRECT,
INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING
WITHOUT LIMITATION LOST PROFITS), HOWEVER CAUSED AND ON ANY THEORY OF
LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING
NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE DATA OR RESULTS,
EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGES.

## 5. Definitions

5.1. "Data" means the material received by a Data Recipient under
this agreement.

5.2. "Data Provider" means any person who is the source of Data
provided under this agreement and in reliance on a Data Recipient's
agreement to its terms.

5.3. "Data Recipient" means any person who receives Data directly
or indirectly from a Data Provider and agrees to the terms of this
agreement.

5.4. "Results" means any outcome obtained by computational analysis
of Data, including for example machine learning models and models'
insights.
```

### PSF-2.0 {#psf-20}

```text
PYTHON SOFTWARE FOUNDATION LICENSE VERSION 2

1. This LICENSE AGREEMENT is between the Python Software Foundation
("PSF"), and the Individual or Organization ("Licensee") accessing and
otherwise using this software ("Python") in source or binary form and
its associated documentation.

2. Subject to the terms and conditions of this License Agreement, PSF hereby
grants Licensee a nonexclusive, royalty-free, world-wide license to reproduce,
analyze, test, perform and/or display publicly, prepare derivative works,
distribute, and otherwise use Python alone or in any derivative version,
provided, however, that PSF's License Agreement and PSF's notice of copyright,
i.e., "Copyright (c) 2001, 2002, 2003, 2004, 2005, 2006, 2007, 2008, 2009, 2010,
2011, 2012, 2013, 2014, 2015, 2016, 2017, 2018, 2019 Python Software Foundation;
All Rights Reserved" are retained in Python alone or in any derivative version
prepared by Licensee.

3. In the event Licensee prepares a derivative work that is based on
or incorporates Python or any part thereof, and wants to make
the derivative work available to others as provided herein, then
Licensee hereby agrees to include in any such work a brief summary of
the changes made to Python.

4. PSF is making Python available to Licensee on an "AS IS"
basis.  PSF MAKES NO REPRESENTATIONS OR WARRANTIES, EXPRESS OR
IMPLIED.  BY WAY OF EXAMPLE, BUT NOT LIMITATION, PSF MAKES NO AND
DISCLAIMS ANY REPRESENTATION OR WARRANTY OF MERCHANTABILITY OR FITNESS
FOR ANY PARTICULAR PURPOSE OR THAT THE USE OF PYTHON WILL NOT
INFRINGE ANY THIRD PARTY RIGHTS.

5. PSF SHALL NOT BE LIABLE TO LICENSEE OR ANY OTHER USERS OF PYTHON
FOR ANY INCIDENTAL, SPECIAL, OR CONSEQUENTIAL DAMAGES OR LOSS AS
A RESULT OF MODIFYING, DISTRIBUTING, OR OTHERWISE USING PYTHON,
OR ANY DERIVATIVE THEREOF, EVEN IF ADVISED OF THE POSSIBILITY THEREOF.

6. This License Agreement will automatically terminate upon a material
breach of its terms and conditions.

7. Nothing in this License Agreement shall be deemed to create any
relationship of agency, partnership, or joint venture between PSF and
Licensee.  This License Agreement does not grant permission to use PSF
trademarks or trade name in a trademark sense to endorse or promote
products or services of Licensee, or any third party.

8. By copying, installing or otherwise using Python, Licensee
agrees to be bound by the terms and conditions of this License
Agreement.
```

### MPL-2.0 {#mpl-20}

```text
Mozilla Public License Version 2.0
==================================

1. Definitions
--------------

1.1. "Contributor"
    means each individual or legal entity that creates, contributes to
    the creation of, or owns Covered Software.

1.2. "Contributor Version"
    means the combination of the Contributions of others (if any) used
    by a Contributor and that particular Contributor's Contribution.

1.3. "Contribution"
    means Covered Software of a particular Contributor.

1.4. "Covered Software"
    means Source Code Form to which the initial Contributor has attached
    the notice in Exhibit A, the Executable Form of such Source Code
    Form, and Modifications of such Source Code Form, in each case
    including portions thereof.

1.5. "Incompatible With Secondary Licenses"
    means

    (a) that the initial Contributor has attached the notice described
        in Exhibit B to the Covered Software; or

    (b) that the Covered Software was made available under the terms of
        version 1.1 or earlier of the License, but not also under the
        terms of a Secondary License.

1.6. "Executable Form"
    means any form of the work other than Source Code Form.

1.7. "Larger Work"
    means a work that combines Covered Software with other material, in
    a separate file or files, that is not Covered Software.

1.8. "License"
    means this document.

1.9. "Licensable"
    means having the right to grant, to the maximum extent possible,
    whether at the time of the initial grant or subsequently, any and
    all of the rights conveyed by this License.

1.10. "Modifications"
    means any of the following:

    (a) any file in Source Code Form that results from an addition to,
        deletion from, or modification of the contents of Covered
        Software; or

    (b) any new file in Source Code Form that contains any Covered
        Software.

1.11. "Patent Claims" of a Contributor
    means any patent claim(s), including without limitation, method,
    process, and apparatus claims, in any patent Licensable by such
    Contributor that would be infringed, but for the grant of the
    License, by the making, using, selling, offering for sale, having
    made, import, or transfer of either its Contributions or its
    Contributor Version.

1.12. "Secondary License"
    means either the GNU General Public License, Version 2.0, the GNU
    Lesser General Public License, Version 2.1, the GNU Affero General
    Public License, Version 3.0, or any later versions of those
    licenses.

1.13. "Source Code Form"
    means the form of the work preferred for making modifications.

1.14. "You" (or "Your")
    means an individual or a legal entity exercising rights under this
    License. For legal entities, "You" includes any entity that
    controls, is controlled by, or is under common control with You. For
    purposes of this definition, "control" means (a) the power, direct
    or indirect, to cause the direction or management of such entity,
    whether by contract or otherwise, or (b) ownership of more than
    fifty percent (50%) of the outstanding shares or beneficial
    ownership of such entity.

2. License Grants and Conditions
--------------------------------

2.1. Grants

Each Contributor hereby grants You a world-wide, royalty-free,
non-exclusive license:

(a) under intellectual property rights (other than patent or trademark)
    Licensable by such Contributor to use, reproduce, make available,
    modify, display, perform, distribute, and otherwise exploit its
    Contributions, either on an unmodified basis, with Modifications, or
    as part of a Larger Work; and

(b) under Patent Claims of such Contributor to make, use, sell, offer
    for sale, have made, import, and otherwise transfer either its
    Contributions or its Contributor Version.

2.2. Effective Date

The licenses granted in Section 2.1 with respect to any Contribution
become effective for each Contribution on the date the Contributor first
distributes such Contribution.

2.3. Limitations on Grant Scope

The licenses granted in this Section 2 are the only rights granted under
this License. No additional rights or licenses will be implied from the
distribution or licensing of Covered Software under this License.
Notwithstanding Section 2.1(b) above, no patent license is granted by a
Contributor:

(a) for any code that a Contributor has removed from Covered Software;
    or

(b) for infringements caused by: (i) Your and any other third party's
    modifications of Covered Software, or (ii) the combination of its
    Contributions with other software (except as part of its Contributor
    Version); or

(c) under Patent Claims infringed by Covered Software in the absence of
    its Contributions.

This License does not grant any rights in the trademarks, service marks,
or logos of any Contributor (except as may be necessary to comply with
the notice requirements in Section 3.4).

2.4. Subsequent Licenses

No Contributor makes additional grants as a result of Your choice to
distribute the Covered Software under a subsequent version of this
License (see Section 10.2) or under the terms of a Secondary License (if
permitted under the terms of Section 3.3).

2.5. Representation

Each Contributor represents that the Contributor believes its
Contributions are its original creation(s) or it has sufficient rights
to grant the rights to its Contributions conveyed by this License.

2.6. Fair Use

This License is not intended to limit any rights You have under
applicable copyright doctrines of fair use, fair dealing, or other
equivalents.

2.7. Conditions

Sections 3.1, 3.2, 3.3, and 3.4 are conditions of the licenses granted
in Section 2.1.

3. Responsibilities
-------------------

3.1. Distribution of Source Form

All distribution of Covered Software in Source Code Form, including any
Modifications that You create or to which You contribute, must be under
the terms of this License. You must inform recipients that the Source
Code Form of the Covered Software is governed by the terms of this
License, and how they can obtain a copy of this License. You may not
attempt to alter or restrict the recipients' rights in the Source Code
Form.

3.2. Distribution of Executable Form

If You distribute Covered Software in Executable Form then:

(a) such Covered Software must also be made available in Source Code
    Form, as described in Section 3.1, and You must inform recipients of
    the Executable Form how they can obtain a copy of such Source Code
    Form by reasonable means in a timely manner, at a charge no more
    than the cost of distribution to the recipient; and

(b) You may distribute such Executable Form under the terms of this
    License, or sublicense it under different terms, provided that the
    license for the Executable Form does not attempt to limit or alter
    the recipients' rights in the Source Code Form under this License.

3.3. Distribution of a Larger Work

You may create and distribute a Larger Work under terms of Your choice,
provided that You also comply with the requirements of this License for
the Covered Software. If the Larger Work is a combination of Covered
Software with a work governed by one or more Secondary Licenses, and the
Covered Software is not Incompatible With Secondary Licenses, this
License permits You to additionally distribute such Covered Software
under the terms of such Secondary License(s), so that the recipient of
the Larger Work may, at their option, further distribute the Covered
Software under the terms of either this License or such Secondary
License(s).

3.4. Notices

You may not remove or alter the substance of any license notices
(including copyright notices, patent notices, disclaimers of warranty,
or limitations of liability) contained within the Source Code Form of
the Covered Software, except that You may alter any license notices to
the extent required to remedy known factual inaccuracies.

3.5. Application of Additional Terms

You may choose to offer, and to charge a fee for, warranty, support,
indemnity or liability obligations to one or more recipients of Covered
Software. However, You may do so only on Your own behalf, and not on
behalf of any Contributor. You must make it absolutely clear that any
such warranty, support, indemnity, or liability obligation is offered by
You alone, and You hereby agree to indemnify every Contributor for any
liability incurred by such Contributor as a result of warranty, support,
indemnity or liability terms You offer. You may include additional
disclaimers of warranty and limitations of liability specific to any
jurisdiction.

4. Inability to Comply Due to Statute or Regulation
---------------------------------------------------

If it is impossible for You to comply with any of the terms of this
License with respect to some or all of the Covered Software due to
statute, judicial order, or regulation then You must: (a) comply with
the terms of this License to the maximum extent possible; and (b)
describe the limitations and the code they affect. Such description must
be placed in a text file included with all distributions of the Covered
Software under this License. Except to the extent prohibited by statute
or regulation, such description must be sufficiently detailed for a
recipient of ordinary skill to be able to understand it.

5. Termination
--------------

5.1. The rights granted under this License will terminate automatically
if You fail to comply with any of its terms. However, if You become
compliant, then the rights granted under this License from a particular
Contributor are reinstated (a) provisionally, unless and until such
Contributor explicitly and finally terminates Your grants, and (b) on an
ongoing basis, if such Contributor fails to notify You of the
non-compliance by some reasonable means prior to 60 days after You have
come back into compliance. Moreover, Your grants from a particular
Contributor are reinstated on an ongoing basis if such Contributor
notifies You of the non-compliance by some reasonable means, this is the
first time You have received notice of non-compliance with this License
from such Contributor, and You become compliant prior to 30 days after
Your receipt of the notice.

5.2. If You initiate litigation against any entity by asserting a patent
infringement claim (excluding declaratory judgment actions,
counter-claims, and cross-claims) alleging that a Contributor Version
directly or indirectly infringes any patent, then the rights granted to
You by any and all Contributors for the Covered Software under Section
2.1 of this License shall terminate.

5.3. In the event of termination under Sections 5.1 or 5.2 above, all
end user license agreements (excluding distributors and resellers) which
have been validly granted by You or Your distributors under this License
prior to termination shall survive termination.

************************************************************************
*                                                                      *
*  6. Disclaimer of Warranty                                           *
*  -------------------------                                           *
*                                                                      *
*  Covered Software is provided under this License on an "as is"       *
*  basis, without warranty of any kind, either expressed, implied, or  *
*  statutory, including, without limitation, warranties that the       *
*  Covered Software is free of defects, merchantable, fit for a        *
*  particular purpose or non-infringing. The entire risk as to the     *
*  quality and performance of the Covered Software is with You.        *
*  Should any Covered Software prove defective in any respect, You     *
*  (not any Contributor) assume the cost of any necessary servicing,   *
*  repair, or correction. This disclaimer of warranty constitutes an   *
*  essential part of this License. No use of any Covered Software is   *
*  authorized under this License except under this disclaimer.         *
*                                                                      *
************************************************************************

************************************************************************
*                                                                      *
*  7. Limitation of Liability                                          *
*  --------------------------                                          *
*                                                                      *
*  Under no circumstances and under no legal theory, whether tort      *
*  (including negligence), contract, or otherwise, shall any           *
*  Contributor, or anyone who distributes Covered Software as          *
*  permitted above, be liable to You for any direct, indirect,         *
*  special, incidental, or consequential damages of any character      *
*  including, without limitation, damages for lost profits, loss of    *
*  goodwill, work stoppage, computer failure or malfunction, or any    *
*  and all other commercial damages or losses, even if such party      *
*  shall have been informed of the possibility of such damages. This   *
*  limitation of liability shall not apply to liability for death or   *
*  personal injury resulting from such party's negligence to the       *
*  extent applicable law prohibits such limitation. Some               *
*  jurisdictions do not allow the exclusion or limitation of           *
*  incidental or consequential damages, so this exclusion and          *
*  limitation may not apply to You.                                    *
*                                                                      *
************************************************************************

8. Litigation
-------------

Any litigation relating to this License may be brought only in the
courts of a jurisdiction where the defendant maintains its principal
place of business and such litigation shall be governed by laws of that
jurisdiction, without reference to its conflict-of-law provisions.
Nothing in this Section shall prevent a party's ability to bring
cross-claims or counter-claims.

9. Miscellaneous
----------------

This License represents the complete agreement concerning the subject
matter hereof. If any provision of this License is held to be
unenforceable, such provision shall be reformed only to the extent
necessary to make it enforceable. Any law or regulation which provides
that the language of a contract shall be construed against the drafter
shall not be used to construe this License against a Contributor.

10. Versions of the License
---------------------------

10.1. New Versions

Mozilla Foundation is the license steward. Except as provided in Section
10.3, no one other than the license steward has the right to modify or
publish new versions of this License. Each version will be given a
distinguishing version number.

10.2. Effect of New Versions

You may distribute the Covered Software under the terms of the version
of the License under which You originally received the Covered Software,
or under the terms of any subsequent version published by the license
steward.

10.3. Modified Versions

If you create software not governed by this License, and you want to
create a new license for such software, you may create and use a
modified version of this License if you rename the license and remove
any references to the name of the license steward (except to note that
such modified license differs from this License).

10.4. Distributing Source Code Form that is Incompatible With Secondary
Licenses

If You choose to distribute Source Code Form that is Incompatible With
Secondary Licenses under the terms of this version of the License, the
notice described in Exhibit B of this License must be attached.

Exhibit A - Source Code Form License Notice
-------------------------------------------

  This Source Code Form is subject to the terms of the Mozilla Public
  License, v. 2.0. If a copy of the MPL was not distributed with this
  file, You can obtain one at http://mozilla.org/MPL/2.0/.

If it is not possible or desirable to put the notice in a particular
file, then You may include the notice in a location (such as a LICENSE
file in a relevant directory) where a recipient would be likely to look
for such a notice.

You may add additional accurate notices of copyright ownership.

Exhibit B - "Incompatible With Secondary Licenses" Notice
---------------------------------------------------------

  This Source Code Form is "Incompatible With Secondary Licenses", as
  defined by the Mozilla Public License, v. 2.0.
```

### bzip2-1.0.6 {#bzip2-106}

```text
--------------------------------------------------------------------------

The original program, "bzip2", the associated library "libbzip2", and all
documentation, are

Copyright (C) 1996-2021 Julian R Seward.
Copyright (C) 2019-2020 Federico Mena Quintero
Copyright (C) 2021 Micah Snyder

This Rust translation, "libbzip2-rs" is a derived work based on "bzip2" and
"libbzip2", and is Copyright (C) 2024-2025 Trifecta Tech Foundation and contributors

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions
are met:

1. Redistributions of source code must retain the above copyright
   notice, this list of conditions and the following disclaimer.

2. The origin of this software must not be misrepresented; you must
   not claim that you wrote the original software.  If you use this
   software in a product, an acknowledgment in the product
   documentation would be appreciated but is not required.

3. Altered source versions must be plainly marked as such, and must
   not be misrepresented as being the original software.

4. The name of the author may not be used to endorse or promote
   products derived from this software without specific prior written
   permission.

THIS SOFTWARE IS PROVIDED BY THE AUTHOR ``AS IS'' AND ANY EXPRESS
OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
ARE DISCLAIMED.  IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY
DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE
GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING
NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

Julian Seward, jseward@acm.org
bzip2/libbzip2 version 1.1.0 of 6 September 2010

--------------------------------------------------------------------------
```
