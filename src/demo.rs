//! The bundled demo game — a Rust port of the demo mod that ships with The
//! Modding Tree (js/Demo/). Mod authors replace this module with their own
//! layers; it doubles as the executable example of every feature.

use crate::defs::*;
use crate::format::{dec, dnum, format, format_whole, Decimal};
use crate::game::{Game, PVal};

pub fn game_defs() -> GameDefs {
    GameDefs {
        mod_info: ModInfo {
            name: "The Modding Tree",
            id: "modbase",
            points_name: "points",
            initial_start_points: Decimal::from(10),
            offline_limit: 1.0,
            allow_small: false,
            author: "",
        },
        version: "2.6.6",
        version_name: "Fixed Reality",
        win_text: "Congratulations! You have reached the end and beaten this game, but for now...",
        changelog: "<h1>Changelog:</h1><br><h3>v2.6.6</h3><br>- Rust edition.",
        start_tab: "c",
        start_nav_tab: "tree-tab",
        show_tree: true,
        can_gen_points: view(|g| g.has_upgrade("c", 11)),
        get_point_gen: view(|g| {
            if !g.has_upgrade("c", 11) {
                return Decimal::zero();
            }
            let mut gain = Decimal::one();
            if g.has_upgrade("c", 12) {
                gain = gain * g.upgrade_effect("c", 12).dec();
            }
            gain
        }),
        is_endgame: view(|g| g.player.points >= Decimal::from(11)),
        display_things: vec![
            view(|g| {
                if g.player.points == Decimal::from(69) {
                    "Tee hee!".to_string()
                } else {
                    String::new()
                }
            }),
            view(|g| {
                if g.lstate("f").points > Decimal::one() {
                    format!(
                        "You have {} farm points. (Which do nothing.)",
                        g.lstate("f").points
                    )
                } else {
                    String::new()
                }
            }),
            view(|g| {
                if g.in_challenge("c", 11) {
                    "The game is currently <h1>0%</h1> harder.".to_string()
                } else {
                    String::new()
                }
            }),
        ],
        background_style: view(|_| String::new()),
        max_tick_length: 3600.0,
        added_player_data: vec![
            ("weather", PVal::Str("Yes".to_string())),
            ("happiness", PVal::Dec(Decimal::from(72))),
        ],
        layers: vec![
            layer_c(),
            layer_f(),
            layer_a(),
            node_spook(),
            node_g(),
            node_h(),
            layer_tree_tab(),
            layer_info_tab(),
            layer_options_tab(),
            layer_changelog_tab(),
        ],
    }
}

// ---------------------------------------------------------------------------
// The "c" layer (js/Demo/layers/c.js)
// ---------------------------------------------------------------------------

fn layer_c() -> LayerDef {
    let mut c = LayerDef {
        id: "c",
        name: Some("Candies"),
        symbol: Some("C"),
        color: "#4BDC13",
        position: Some(0.0),
        row: Row::Tree(0),
        resource: "lollipops",
        base_resource: "points",
        base_amount: view(|g| g.player.points),
        requires: DecOrFn::Dec(Decimal::from(10)),
        prestige_type: PrestigeType::Normal,
        exponent: view(|_| dnum(0.5)),
        base: view(|_| Decimal::from(5)),
        softcap: view(|_| dec("1e100")),
        softcap_power: view(|_| dnum(0.5)),
        gain_mult: view(|g| {
            let mut mult = Decimal::one();
            if g.has_upgrade("c", 166) {
                mult = mult * Decimal::from(2);
            }
            if g.has_upgrade("c", 120) {
                mult = mult * g.upgrade_effect("c", 120).dec();
            }
            mult
        }),
        reset_description: Some("Melt your points into "),
        glow_color: Some("orange"),
        marked: view(|_| Some("resources/discord.png".to_string())),
        node_style: view(|_| "color:#3325CC;text-decoration:underline;font-family:cursive".to_string()),
        tooltip: view(|g| {
            let mut t = format!("{} {}", format_whole(g.lstate("c").points), g.ltmp("c").resource);
            if g.get_buyable_amount("c", "11") > Decimal::zero() {
                t += &format!(
                    "<br><i><br><br><br>{} Exhancers</i>",
                    format_whole(g.get_buyable_amount("c", "11"))
                );
            }
            Some(t)
        }),
        should_notify: view(|g| g.get_buyable_amount("c", "11") == Decimal::one()),
        effect: Some(view(|g| {
            let points = g.lstate("c").points;
            EffectVal::Map(vec![
                ("waffleBoost", points.pow(dnum(0.2))),
                ("icecreamCap", points * Decimal::from(10)),
            ])
        })),
        effect_description: Some(view(|g| {
            let mut waffle = g.ltmp("c").effect.as_ref().map(|e| e.get("waffleBoost")).unwrap_or_else(Decimal::zero);
            waffle = waffle * g.buyable_effect("c", "11").get("first");
            let cap = g
                .ltmp("c")
                .effect
                .as_ref()
                .map(|e| e.get("icecreamCap"))
                .unwrap_or_else(Decimal::zero);
            format!(
                "which are boosting waffles by {} and increasing the Ice Cream cap by {}",
                format(waffle, 2, false),
                format(cap, 2, false)
            )
        })),
        increase_unlock_order: vec![],
        ..Default::default()
    };

    c.start_data = vec![
        ("beep", PVal::Bool(false)),
        ("thingy", PVal::Str("pointy".to_string())),
        ("otherThingy", PVal::Num(10.0)),
        ("drop", PVal::Str("drip".to_string())),
    ];

    // ----- infobox -----
    c.infoboxes = vec![InfoboxDef {
        id: "coolInfo",
        title: view(|_| "Lore".to_string()),
        title_style: view(|_| "color:#FE0000".to_string()),
        body: view(|_| "DEEP LORE!".to_string()),
        body_style: view(|_| "background-color:#0000EE".to_string()),
        ..Default::default()
    }];

    // ----- milestones -----
    c.milestones = vec![
        MilestoneDef {
            id: 0,
            requirement_description: "3 Lollipops".to_string(),
            done: view(|g| g.lstate("c").best >= Decimal::from(3)),
            effect_description: view(|_| "Unlock the next milestone".to_string()),
            ..Default::default()
        },
        MilestoneDef {
            id: 1,
            requirement_description: "4 Lollipops".to_string(),
            done: view(|g| g.lstate("c").best >= Decimal::from(4)),
            effect_description: view(|_| "You can toggle beep and boop (which do nothing)".to_string()),
            unlocked: view(|g| g.has_milestone("c", 0)),
            toggles: vec![("c", "beep"), ("f", "boop")],
            style: view(|g| {
                if g.has_milestone("c", 1) {
                    "background-color:#1111DD".to_string()
                } else {
                    String::new()
                }
            }),
            ..Default::default()
        },
    ];

    // ----- challenges -----
    c.challenges = vec![ChallengeDef {
        id: 11,
        name: "Fun".to_string(),
        completion_limit: 3,
        challenge_description: view(|g| {
            format!(
                "Makes the game 0% harder<br>{}/3 completions",
                g.challenge_completions("c", 11)
            )
        }),
        unlocked: view(|g| g.lstate("c").best > Decimal::zero()),
        goal: Decimal::from(20),
        goal_description: Some("Have 20 points I guess".to_string()),
        can_complete: Some(view(|g| g.player.points >= Decimal::from(20))),
        reward_effect: None,
        reward_description: Some("Says hi".to_string()),
        counts_as: vec![12, 21],
        ..Default::default()
    }];
    c.challenges[0].reward_effect = Some(view(|g| {
        EffectVal::Dec(
            (g.lstate("c").points + Decimal::one()).tetrate(Some(0.02), None, crate::TetrationMode::Analytic),
        )
    }));
    c.challenges[0].reward_display = Some(view(|g| {
        format!("{}x", format(g.challenge_effect("c", 11).dec(), 2, false))
    }));

    // ----- upgrades -----
    c.upgrades = vec![
        UpgradeDef {
            id: 11,
            title: Some("Generator of Genericness".to_string()),
            description: Some("Gain 1 Point every second.".to_string()),
            cost: DecOrFn::Dec(Decimal::from(1)),
            branches: vec![Branch::Plain("12")],
            tooltip: view(|_| Some("hi".to_string())),
            ..Default::default()
        },
        UpgradeDef {
            id: 12,
            description: Some("Point generation is faster based on your unspent Lollipops.".to_string()),
            cost: DecOrFn::Dec(Decimal::from(1)),
            unlocked: view(|g| g.has_upgrade("c", 11)),
            effect: Some(view(|g| {
                let mut ret = (g.lstate("c").points + Decimal::one()).pow({
                    let ups = &g.lstate("c").upgrades;
                    if ups.contains(&24) {
                        dnum(1.1)
                    } else if ups.contains(&14) {
                        dnum(0.75)
                    } else {
                        dnum(0.5)
                    }
                });
                if ret >= dec("1e20000000") {
                    ret = ret.sqrt() * dec("1e10000000");
                }
                EffectVal::Dec(ret)
            })),
            effect_display: Some(view(|g| {
                format!("{}x", format(g.upgrade_effect("c", 12).dec(), 2, false))
            })),
            ..Default::default()
        },
        UpgradeDef {
            id: 13,
            unlocked: view(|g| g.has_upgrade("c", 12)),
            on_purchase: Some(act(|g| {
                g.lstate_mut("c").unlock_order = 0;
            })),
            style: view(|g| {
                if g.has_upgrade("c", 13) {
                    "background-color:#1111dd".to_string()
                } else if !g.can_afford_upgrade("c", 13) {
                    "background-color:#dd1111".to_string()
                } else {
                    String::new()
                }
            }),
            can_afford: Some(view(|g| g.player.points <= Decimal::from(7))),
            pay: Some(act(|g| {
                g.player.points = g.player.points + Decimal::from(7);
            })),
            full_display: Some(view(|_| {
                "Only buyable with less than 7 points, and gives you 7 more. Unlocks a secret subtab.".to_string()
            })),
            ..Default::default()
        },
        UpgradeDef {
            id: 22,
            title: Some("This upgrade doesn't exist".to_string()),
            description: Some("Or does it?.".to_string()),
            currency: Some(Currency::Buyable { layer: "c", id: "11" }),
            cost: DecOrFn::Dec(Decimal::from(3)),
            ..Default::default()
        },
    ];

    // ----- buyables -----
    c.buyables = BuyablesDef {
        show_respec: true,
        respec_text: "Respec Thingies".to_string(),
        respec_message: "Are you sure? Respeccing these doesn't accomplish much.".to_string(),
        respec: Some(act(|g| {
            let spent = g.lstate("c").spent_on_buyables;
            g.lstate_mut("c").points = g.lstate("c").points + spent;
            g.set_buyable_amount("c", "11", Decimal::zero());
            g.lstate_mut("c").spent_on_buyables = Decimal::zero();
            g.do_reset("c", true);
        })),
        items: vec![BuyableDef {
            id: "11",
            title: Some("Exhancers".to_string()),
            cost: view(|_| {
                // Cost is resolved against the current amount in tmp; this
                // function receives it via the game (see below).
                Decimal::zero()
            }),
            purchase_limit: Decimal::from(4),
            style: view(|_| "height:222px".to_string()),
            can_afford: view(|g| g.lstate("c").points >= g.ltmp("c").buyables.get("11").map(|b| b.cost).unwrap_or_else(Decimal::zero)),
            buy: Some(act(|g| {
                let cost = g.ltmp("c").buyables.get("11").map(|b| b.cost).unwrap_or_else(Decimal::zero);
                g.lstate_mut("c").points = g.lstate("c").points - cost;
                let amount = g.get_buyable_amount("c", "11");
                g.set_buyable_amount("c", "11", amount + Decimal::one());
                g.lstate_mut("c").spent_on_buyables = g.lstate("c").spent_on_buyables + cost;
            })),
            sell_one: Some(act(|g| {
                let amount = g.get_buyable_amount("c", "11");
                if amount <= Decimal::zero() {
                    return;
                }
                let cost = buyable_cost("c", "11", amount - Decimal::one());
                g.set_buyable_amount("c", "11", amount - Decimal::one());
                g.lstate_mut("c").points = g.lstate("c").points + cost;
            })),
            ..Default::default()
        }],
        ..Default::default()
    };
    // The cost/effect functions need the amount they're queried at; wire them
    // through helpers that read the current amount (TMT does the same by
    // wrapping cost/effect in setupBuyables).
    c.buyables.items[0].cost = view(|g| buyable_cost("c", "11", g.get_buyable_amount("c", "11")));
    c.buyables.items[0].effect = view(|g| buyable_effect("c", "11", g.get_buyable_amount("c", "11")));
    c.buyables.items[0].display = view(|g| {
        let bt = g.ltmp("c").buyables.get("11").cloned().unwrap_or_default();
        format!(
            "Cost: {} lollipops\nAmount: {}/4\nAdds + {} things and multiplies stuff by {}",
            format(bt.cost, 2, false),
            g.get_buyable_amount("c", "11"),
            format(bt.effect.get("first"), 2, false),
            format(bt.effect.get("second"), 2, false)
        )
    });

    // ----- bars -----
    c.bars = vec![
        BarDef {
            id: "longBoi",
            direction: Direction::Right,
            width: 300.0,
            height: 30.0,
            progress: view(|g| ((g.player.points + Decimal::one()).log10() / Decimal::from(10)).to_number()),
            display: view(|g| format!("{} / 1e10 points", format(g.player.points, 2, false))),
            fill_style: view(|_| "background-color:#FFFFFF".to_string()),
            base_style: view(|_| "background-color:#696969".to_string()),
            text_style: view(|_| "color:#04e050".to_string()),
            ..Default::default()
        },
        BarDef {
            id: "tallBoi",
            direction: Direction::Up,
            width: 50.0,
            height: 200.0,
            progress: view(|g| (g.player.points / Decimal::from(100)).to_number()),
            display: view(|g| {
                format!(
                    "{}%",
                    format_whole((g.player.points / Decimal::from(1)).min(Decimal::from(100)))
                )
            }),
            fill_style: view(|_| "background-color:#4BEC13".to_string()),
            base_style: view(|_| "background-color:#000000".to_string()),
            text_style: view(|_| "text-shadow:0px 0px 2px #000000".to_string()),
            ..Default::default()
        },
        BarDef {
            id: "flatBoi",
            direction: Direction::Up,
            width: 100.0,
            height: 30.0,
            progress: view(|g| (g.lstate("c").points / Decimal::from(50)).to_number()),
            display: view(|_| String::new()),
            fill_style: view(|_| "background-color:#FE0102".to_string()),
            base_style: view(|_| "background-color:#222222".to_string()),
            ..Default::default()
        },
    ];

    // ----- microtabs -----
    c.microtabs = vec![MicrotabsDef {
        family: "stuff",
        tabs: vec![
            SubtabDef {
                name: "first",
                content: vec![
                    item(Content::Upgrades),
                    item(Content::DisplayText(view(|g| format!("confirmed<br>{}", g.lstate("c").extra_str("drop"))))),
                    item(Content::DropDown(
                        "drop",
                        vec!["drip".to_string(), "drop".to_string()],
                    )),
                ],
                ..Default::default()
            },
            SubtabDef {
                name: "second",
                embed_layer: Some("f"),
                content: vec![
                    item(Content::Upgrade(11)),
                    item(Content::Row(vec![
                        item(Content::Upgrade(11)),
                        item(Content::Blank(None)),
                        item(Content::Blank(None)),
                        item(Content::Upgrade(11)),
                    ])),
                    item(Content::DisplayText(view(|_| "double confirmed".to_string()))),
                ],
                ..Default::default()
            },
        ],
    }];

    // ----- tab format -----
    c.tab_format = Some(TabFormat::Tabs(vec![
        SubtabDef {
            name: "main tab",
            content: vec![
                item(Content::MainDisplay),
                item(Content::PrestigeButton),
                item(Content::ResourceDisplay),
                item(Content::Blank(Some(Size::Height(5.0)))),
                item(Content::RawHtml(view(|_| {
                    "<button onclick=\"TMT.particles('text')\">'HI'</button>".to_string()
                }))),
                item(Content::DisplayText(view(|_| "Name your points!".to_string()))),
                item(Content::TextInput("thingy")),
                styled(
                    Content::DisplayText(view(|g| {
                        format!(
                            "I have {} {} points!",
                            format(g.player.points, 2, false),
                            g.lstate("c").extra_str("thingy")
                        )
                    })),
                    "color:red;font-size:32px;font-family:Comic Sans MS",
                ),
                item(Content::HLine(None)),
                item(Content::Milestones),
                item(Content::Blank(None)),
                item(Content::Upgrades),
                item(Content::Challenges),
            ],
            glow_color: Some("blue"),
            ..Default::default()
        },
        SubtabDef {
            name: "thingies",
            prestige_notify: view(|_| true),
            style: view(|_| "background-color:#222222".to_string()),
            button_style: view(|_| "border-color:orange".to_string()),
            content: vec![
                item(Content::Buyables),
                item(Content::Blank(None)),
                item(Content::Row(vec![
                    item(Content::Toggle("c", "beep")),
                    item(Content::Blank(Some(Size::WH(30.0, 10.0)))),
                    item(Content::DisplayText(view(|_| "Beep".to_string()))),
                    item(Content::Blank(None)),
                    item(Content::VLine(Some(200.0))),
                    item(Content::Column(vec![
                        styled(Content::PrestigeButton, "width:150px;height:80px"),
                        styled(Content::PrestigeButton, "width:100px;height:150px"),
                    ])),
                ])),
                item(Content::Blank(None)),
                item(Content::DisplayImage("resources/discord.png".to_string())),
            ],
            ..Default::default()
        },
        SubtabDef {
            name: "jail",
            style: view(|_| "background-color:#222222".to_string()),
            content: vec![
                item(Content::Infobox("coolInfo")),
                item(Content::Bar("longBoi")),
                item(Content::Blank(None)),
                item(Content::Row(vec![
                    styled(
                        Content::Column(vec![
                            item(Content::DisplayText(view(|_| "Sugar level:".to_string()))),
                            item(Content::Blank(None)),
                            item(Content::Bar("tallBoi")),
                        ]),
                        "background-color:#555555;padding:15px",
                    ),
                    item(Content::Blank(None)),
                    item(Content::Column(vec![
                        item(Content::DisplayText(view(|_| "idk".to_string()))),
                        item(Content::Blank(Some(Size::WH(0.0, 50.0)))),
                        item(Content::Bar("flatBoi")),
                    ])),
                ])),
                item(Content::Blank(None)),
                item(Content::DisplayText(view(|_| "It's jail because \"bars\"! So funny! Ha ha!".to_string()))),
                item(Content::Tree(vec![
                    vec![
                        TreeNode::Layer("f"),
                        TreeNode::Layer("c"),
                    ],
                    vec![
                        TreeNode::Layer("g"),
                        TreeNode::Layer("spook"),
                        TreeNode::Layer("h"),
                    ],
                ])),
            ],
            ..Default::default()
        },
        SubtabDef {
            name: "illuminati",
            unlocked: view(|g| g.has_upgrade("c", 13)),
            content: vec![
                item(Content::RawHtml(view(|_| "<h1> C O N F I R M E D </h1>".to_string()))),
                item(Content::Blank(None)),
                styled(
                    Content::Microtabs("stuff"),
                    "width:600px;height:350px;background-color:brown;border-style:solid",
                ),
                item(Content::DisplayText(view(|_| "Adjust how many points H gives you!".to_string()))),
                item(Content::Slider("otherThingy", 1.0, 30.0)),
                item(Content::Blank(None)),
                item(Content::ThingTree(
                    "upgrade",
                    vec![vec![11], vec![12, 22, 22, 11]],
                )),
            ],
            ..Default::default()
        },
    ]));

    // ----- component styles -----
    c.component_styles.challenge = view(|_| "height:200px".to_string());
    c.component_styles.prestige_button = view(|_| "color:#AA66AA".to_string());

    // ----- callbacks -----
    c.do_reset = Some(act2_str(|g, resetting| {
        let resetting_row = g.ltmp(resetting).row.number();
        if resetting_row > 0 {
            crate::player::layer_data_reset(&mut g.player, &g.defs, "c", &["points"]);
        }
    }));

    c.hotkeys = vec![
        HotkeyDef {
            key: "c",
            description: "C: reset for lollipops or whatever".to_string(),
            on_press: act(|g| {
                if g.get_can_reset("c") {
                    g.do_reset("c", false);
                }
            }),
            unlocked: view(|_| true),
        },
        HotkeyDef {
            key: "ctrl+c",
            description: "Ctrl+c: respec things".to_string(),
            on_press: act(|g| g.respec_buyables("c")),
            unlocked: view(|g| g.has_upgrade("c", 22)),
        },
    ];

    c
}

/// c's buyable cost formula (TMT cost(x)).
fn buyable_cost(_layer: &str, _id: &str, x: Decimal) -> Decimal {
    let mut x = x;
    if x >= Decimal::from(25) {
        x = (x.pow(Decimal::from(2)) / Decimal::from(25)).floor();
    }
    let cost = Decimal::from(2).pow(x.pow(dnum(1.5)));
    cost.floor()
}

/// c's buyable effect formula (TMT effect(x)).
fn buyable_effect(_layer: &str, _id: &str, x: Decimal) -> EffectVal {
    let first = if x >= Decimal::zero() {
        Decimal::from(25).pow(x.pow(dnum(1.1)))
    } else {
        dnum(1.0 / 25.0).pow((-x).pow(dnum(1.1)))
    };
    let second = if x >= Decimal::zero() {
        x.pow(dnum(0.8))
    } else {
        (-x).pow(dnum(0.8)) * Decimal::from(-1)
    };
    EffectVal::Map(vec![("first", first), ("second", second)])
}

// ---------------------------------------------------------------------------
// The "f" layer (js/Demo/layers/f.js)
// ---------------------------------------------------------------------------

fn layer_f() -> LayerDef {
    let mut f = LayerDef {
        id: "f",
        color: "#FE0102",
        resource: "farm points",
        base_resource: "points",
        base_amount: view(|g| g.player.points),
        requires: DecOrFn::Fn(view(|_| Decimal::from(10))),
        prestige_type: PrestigeType::Static,
        exponent: view(|_| dnum(0.5)),
        base: view(|_| Decimal::from(3)),
        round_up_cost: true,
        can_buy_max: view(|_| Some(false)),
        row: Row::Tree(1),
        branches: vec![Branch::Plain("c")],
        tooltip_locked: view(|g| {
            Some(format!(
                "This weird farmer dinosaur will only see you if you have at least {} points. You only have {}",
                g.ltmp("f").requires,
                format_whole(g.player.points)
            ))
        }),
        prestige_button_text: Some(view(|g| {
            let lt = g.ltmp("f");
            format!(
                "Hi! I'm a <u>weird dinosaur</u> and I'll give you a Farm Point in exchange for all of your points and lollipops! (At least {} points)",
                format_whole(lt.next_at)
            )
        })),
        ..Default::default()
    };

    f.start_data = vec![
        ("boop", PVal::Bool(false)),
        ("clickable:11", PVal::Str("Start".to_string())),
    ];
    f.start_unlocked = false;

    f.infoboxes = vec![InfoboxDef {
        id: "coolInfo",
        title: view(|_| "Lore".to_string()),
        title_style: view(|_| "color:#FE0000".to_string()),
        body: view(|_| "DEEP LORE!".to_string()),
        body_style: view(|_| "background-color:#0000EE".to_string()),
        ..Default::default()
    }];

    f.midsection = vec![
        item(Content::Blank(None)),
        item(Content::DisplayImage(
            "https://images.beano.com/store/24ab3094eb95e5373bca1ccd6f330d4406db1d1f517fc4170b32e146f806d?auto=compress%2Cformat&dpr=1&w=390".to_string(),
        )),
        item(Content::DisplayText(view(|_| "Bork bork!".to_string()))),
    ];

    f.clickables = ClickablesDef {
        master_button_press: Some(act(|g| {
            let state = g.get_clickable_state("f", "11");
            if state.as_str_val() == "Borkened..." {
                g.set_clickable_state("f", "11", PVal::Str("Start".to_string()));
            }
        })),
        master_button_text: view(|g| {
            if g.get_clickable_state("f", "11").as_str_val() == "Borkened..." {
                "Fix the clickable!".to_string()
            } else {
                "Does nothing".to_string()
            }
        }),
        items: vec![ClickableDef {
            id: "11",
            title: Some("Clicky clicky!".to_string()),
            display: view(|g| {
                format!("Current state:<br>{}", g.get_clickable_state("f", "11").as_str_val())
            }),
            can_click: view(|g| g.get_clickable_state("f", "11").as_str_val() != "Borkened..."),
            on_click: Some(act(|g| {
                let next = match g.get_clickable_state("f", "11").as_str_val().as_str() {
                    "Start" => "A new state!",
                    "A new state!" => "Keep going!",
                    "Keep going!" => "Maybe that's a bit too far...",
                    "Maybe that's a bit too far..." => {
                        g.make_particles(
                            "image",
                            "resources/options_wheel.png".to_string(),
                            4,
                            r#"{"spread":20,"gravity":2,"time":3,"rotation":25,"speed":20,"offset":30,"fadeInTime":1}"#.to_string(),
                        );
                        "Borkened..."
                    }
                    _ => "Start",
                };
                g.set_clickable_state("f", "11", PVal::Str(next.to_string()));
            })),
            style: view(|g| {
                match g.get_clickable_state("f", "11").as_str_val().as_str() {
                    "Start" => "background-color:green",
                    "A new state!" => "background-color:yellow",
                    "Keep going!" => "background-color:orange",
                    "Maybe that's a bit too far..." => "background-color:red",
                    _ => "",
                }
                .to_string()
            }),
            ..Default::default()
        }],
        ..Default::default()
    };

    f
}

// ---------------------------------------------------------------------------
// The "a" layer (js/Demo/layers/a.js) — side layer with achievements & grid
// ---------------------------------------------------------------------------

fn layer_a() -> LayerDef {
    let mut a = LayerDef {
        id: "a",
        color: "yellow",
        resource: "achievement power",
        row: Row::Side,
        tooltip: view(|_| Some("Achievements".to_string())),
        midsection: vec![item(Content::Grid), item(Content::Blank(None))],
        ..Default::default()
    };

    a.achievements = vec![
        AchievementDef {
            id: 11,
            name: "Get me!".to_string(),
            image: Some("resources/discord.png".to_string()),
            done: view(|_| true),
            goal_tooltip: Some("How did this happen?".to_string()),
            done_tooltip: Some("You did it!".to_string()),
            ..Default::default()
        },
        AchievementDef {
            id: 12,
            name: "Impossible!".to_string(),
            done: view(|_| false),
            goal_tooltip: Some("Mwahahaha!".to_string()),
            done_tooltip: Some("HOW????".to_string()),
            text_style: "color:#04e050".to_string(),
            ..Default::default()
        },
        AchievementDef {
            id: 13,
            name: "EIEIO".to_string(),
            done: view(|g| g.lstate("f").points >= Decimal::one()),
            tooltip: Some(
                "Get a farm point.\n\nReward: The dinosaur is now your friend (you can max Farm Points).".to_string(),
            ),
            ..Default::default()
        },
    ];

    let get_style = |g: &Game, id: i64| -> String {
        let data = g.get_grid_data("a", id).as_num();
        format!("background-color:#{}", (data * 1234.0 % 999999.0) as i64)
    };
    a.grid = Some(GridDef {
        rows: 2,
        cols: 2,
        max_rows: 2,
        max_cols: 2,
        get_start_data: std::rc::Rc::new(|id| PVal::Num(id as f64)),
        get_unlocked: view2(|_, _| true),
        get_can_click: view2(|g, _| g.player.points == Decimal::from(10)),
        get_style: view2(move |g, id| get_style(g, id)),
        on_click: Some(act2_num(|g, id| {
            let data = g.get_grid_data("a", id).as_num();
            g.set_grid_data("a", id, PVal::Num(data + 1.0));
        })),
        get_title: view2(|_, id| format!("Gridable #{}", id)),
        get_display: view2(|g, id| g.get_grid_data("a", id).as_str_val()),
        get_tooltip: view2(|_, _| None),
        on_hold: None,
    });

    a
}

// ---------------------------------------------------------------------------
// Tree nodes (js/Demo/demoTree.js)
// ---------------------------------------------------------------------------

fn node_spook() -> LayerDef {
    LayerDef {
        id: "spook",
        is_layer: false,
        row: Row::Tree(1),
        layer_shown: view(|_| LayerShown::Ghost),
        ..Default::default()
    }
}

fn node_g() -> LayerDef {
    LayerDef {
        id: "g",
        is_layer: false,
        symbol: Some("TH"),
        color: "#6d3678",
        row: Row::Tree(1),
        position: Some(0.0),
        branches: vec![Branch::Colored("c", "red", Some(4.0))],
        node_can_click: Some(view(|g| g.player.points >= Decimal::from(10))),
        tooltip: view(|_| Some("Thanos your points".to_string())),
        tooltip_locked: view(|_| Some("Thanos your points".to_string())),
        node_on_click: Some(act(|g| {
            g.player.points = g.player.points / Decimal::from(2);
        })),
        ..Default::default()
    }
}

fn node_h() -> LayerDef {
    LayerDef {
        id: "h",
        is_layer: false,
        row: Row::Side,
        position: Some(1.0),
        branches: vec![Branch::Plain("g")],
        node_can_click: Some(view(|g| g.player.points < dnum(g.lstate("c").extra_num("otherThingy")))),
        tooltip: view(|g| {
            Some(format!(
                "Restore your points to {}",
                g.lstate("c").extra_num("otherThingy")
            ))
        }),
        tooltip_locked: view(|g| {
            Some(format!(
                "Restore your points to {}",
                g.lstate("c").extra_num("otherThingy")
            ))
        }),
        node_on_click: Some(act(|g| {
            let target = dnum(g.lstate("c").extra_num("otherThingy"));
            g.player.points = target;
        })),
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// System tabs
// ---------------------------------------------------------------------------

fn layer_tree_tab() -> LayerDef {
    LayerDef {
        id: "tree-tab",
        // TMT leaves tree-tab's row undefined (excluded from the tree itself).
        row: Row::Otherside,
        tab_format: Some(TabFormat::Array(vec![item(Content::Tree(tree_layout()))])),
        left_tab: true,
        style: view(|_| "background-color:#222222".to_string()),
        ..Default::default()
    }
}

fn tree_layout() -> Vec<Vec<TreeNode>> {
    // TMT sorts same-position layers alphabetically: f, g, spook.
    vec![
        vec![TreeNode::Layer("c")],
        vec![
            TreeNode::Layer("f"),
            TreeNode::Layer("g"),
            TreeNode::Layer("spook"),
        ],
    ]
}

fn layer_info_tab() -> LayerDef {
    LayerDef {
        id: "info-tab",
        tab_format: Some(TabFormat::Array(vec![item(Content::InfoTab)])),
        row: Row::Otherside,
        ..Default::default()
    }
}

fn layer_options_tab() -> LayerDef {
    LayerDef {
        id: "options-tab",
        tab_format: Some(TabFormat::Array(vec![item(Content::OptionsTab)])),
        row: Row::Otherside,
        ..Default::default()
    }
}

fn layer_changelog_tab() -> LayerDef {
    LayerDef {
        id: "changelog-tab",
        tab_format: Some(TabFormat::Array(vec![item(Content::Changelog)])),
        row: Row::Otherside,
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// Small callback constructor helpers
// ---------------------------------------------------------------------------

pub fn act2_str(f: impl Fn(&mut Game, &str) + 'static) -> Act2 {
    std::rc::Rc::new(move |g, arg| {
        if let Arg::Str(s) = arg {
            f(g, &s);
        } else {
            f(g, "");
        }
    })
}

pub fn act2_num(f: impl Fn(&mut Game, i64) + 'static) -> Act2 {
    std::rc::Rc::new(move |g, arg| {
        let n = match arg {
            Arg::Num(n) => n as i64,
            _ => 0,
        };
        f(g, n);
    })
}
