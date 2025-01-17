use std::collections::BTreeSet;

use geom::ArrowCap;
use map_gui::render::{DrawOptions, BIG_ARROW_THICKNESS};
use map_gui::ID;
use map_model::{IntersectionCluster, IntersectionID};
use widgetry::tools::PopupMsg;
use widgetry::{
    Color, DrawBaselayer, Drawable, EventCtx, GeomBatch, GfxCtx, HorizontalAlignment, Key, Line,
    Panel, SimpleState, State, Text, TextExt, Toggle, VerticalAlignment, Widget,
};

use crate::app::{App, ShowEverything, Transition};
use crate::common::CommonState;

pub struct UberTurnPicker {
    members: BTreeSet<IntersectionID>,
}

impl UberTurnPicker {
    pub fn new_state(ctx: &mut EventCtx, app: &App, i: IntersectionID) -> Box<dyn State<App>> {
        let mut members = BTreeSet::new();
        if let Some(list) = IntersectionCluster::autodetect(i, &app.primary.map) {
            members.extend(list);
        } else {
            members.insert(i);
        }

        let panel = Panel::new_builder(Widget::col(vec![
            Widget::row(vec![
                Line("Select multiple intersections")
                    .small_heading()
                    .into_widget(ctx),
                ctx.style().btn_close_widget(ctx),
            ]),
            ctx.style()
                .btn_outline
                .text("View uber-turns")
                .hotkey(Key::Enter)
                .build_def(ctx),
            ctx.style()
                .btn_outline
                .text("Detect all clusters")
                .hotkey(Key::D)
                .build_def(ctx),
        ]))
        .aligned(HorizontalAlignment::Center, VerticalAlignment::Top)
        .build(ctx);
        <dyn SimpleState<_>>::new_state(panel, Box::new(UberTurnPicker { members }))
    }
}

impl SimpleState<App> for UberTurnPicker {
    fn on_click(
        &mut self,
        ctx: &mut EventCtx,
        app: &mut App,
        x: &str,
        _: &mut Panel,
    ) -> Transition {
        match x {
            "close" => Transition::Pop,
            "View uber-turns" => {
                if self.members.len() < 2 {
                    return Transition::Push(PopupMsg::new_state(
                        ctx,
                        "Error",
                        vec!["Select at least two intersections"],
                    ));
                }
                Transition::Replace(UberTurnViewer::new_state(
                    ctx,
                    app,
                    self.members.clone(),
                    0,
                    true,
                ))
            }
            "Detect all clusters" => {
                self.members.clear();
                for ic in IntersectionCluster::find_all(&app.primary.map) {
                    self.members.extend(ic.members);
                }
                Transition::Keep
            }
            _ => unreachable!(),
        }
    }

    fn on_mouseover(&mut self, ctx: &mut EventCtx, app: &mut App) {
        app.primary.current_selection = app.mouseover_unzoomed_intersections(ctx);
    }
    fn other_event(&mut self, ctx: &mut EventCtx, app: &mut App) -> Transition {
        ctx.canvas_movement();
        if let Some(ID::Intersection(i)) = app.primary.current_selection {
            if !self.members.contains(&i) && app.per_obj.left_click(ctx, "add this intersection") {
                self.members.insert(i);
            } else if self.members.contains(&i)
                && app.per_obj.left_click(ctx, "remove this intersection")
            {
                self.members.remove(&i);
            }
        }
        Transition::Keep
    }

    fn draw(&self, g: &mut GfxCtx, app: &App) {
        CommonState::draw_osd(g, app);

        let mut batch = GeomBatch::new();
        for i in &self.members {
            batch.push(
                Color::RED.alpha(0.8),
                app.primary.map.get_i(*i).polygon.clone(),
            );
        }
        let draw = g.upload(batch);
        g.redraw(&draw);
    }
}

struct UberTurnViewer {
    draw: Drawable,
    ic: IntersectionCluster,
    idx: usize,
    legal_turns: bool,
}

impl UberTurnViewer {
    pub fn new_state(
        ctx: &mut EventCtx,
        app: &mut App,
        members: BTreeSet<IntersectionID>,
        idx: usize,
        legal_turns: bool,
    ) -> Box<dyn State<App>> {
        app.primary.current_selection = None;
        let map = &app.primary.map;

        let (ic1, ic2) = IntersectionCluster::new(members, map);
        let ic = if legal_turns { ic1 } else { ic2 };

        let mut batch = GeomBatch::new();
        for i in &ic.members {
            batch.push(Color::BLUE.alpha(0.5), map.get_i(*i).polygon.clone());
        }
        if !ic.uber_turns.is_empty() {
            let ut = &ic.uber_turns[idx];
            batch.push(
                Color::RED,
                ut.geom(map)
                    .make_arrow(BIG_ARROW_THICKNESS, ArrowCap::Triangle),
            );
        }

        let panel = Panel::new_builder(Widget::col(vec![
            Widget::row(vec![
                Line("Uber-turn viewer").small_heading().into_widget(ctx),
                Widget::vert_separator(ctx, 50.0),
                ctx.style()
                    .btn_prev()
                    .disabled(idx == 0)
                    .hotkey(Key::LeftArrow)
                    .build_widget(ctx, "previous uber-turn"),
                Text::from(Line(format!("{}/{}", idx + 1, ic.uber_turns.len())).secondary())
                    .into_widget(ctx)
                    .centered_vert(),
                ctx.style()
                    .btn_next()
                    .disabled(ic.uber_turns.is_empty() || idx == ic.uber_turns.len() - 1)
                    .hotkey(Key::RightArrow)
                    .build_widget(ctx, "next uber-turn"),
                ctx.style().btn_close_widget(ctx),
            ]),
            Widget::row(vec![
                Toggle::choice(
                    ctx,
                    "legal / illegal movements",
                    "legal",
                    "illegal",
                    None,
                    legal_turns,
                ),
                "movements".text_widget(ctx),
            ]),
        ]))
        .aligned(HorizontalAlignment::Center, VerticalAlignment::Top)
        .build(ctx);
        <dyn SimpleState<_>>::new_state(
            panel,
            Box::new(UberTurnViewer {
                draw: ctx.upload(batch),
                ic,
                idx,
                legal_turns,
            }),
        )
    }
}

impl SimpleState<App> for UberTurnViewer {
    fn on_click(
        &mut self,
        ctx: &mut EventCtx,
        app: &mut App,
        x: &str,
        _: &mut Panel,
    ) -> Transition {
        match x {
            "close" => Transition::Pop,
            "previous uber-turn" => Transition::Replace(UberTurnViewer::new_state(
                ctx,
                app,
                self.ic.members.clone(),
                self.idx - 1,
                self.legal_turns,
            )),
            "next uber-turn" => Transition::Replace(UberTurnViewer::new_state(
                ctx,
                app,
                self.ic.members.clone(),
                self.idx + 1,
                self.legal_turns,
            )),
            _ => unreachable!(),
        }
    }
    fn panel_changed(
        &mut self,
        ctx: &mut EventCtx,
        app: &mut App,
        panel: &mut Panel,
    ) -> Option<Transition> {
        Some(Transition::Replace(UberTurnViewer::new_state(
            ctx,
            app,
            self.ic.members.clone(),
            0,
            panel.is_checked("legal / illegal movements"),
        )))
    }

    fn other_event(&mut self, ctx: &mut EventCtx, _: &mut App) -> Transition {
        ctx.canvas_movement();
        Transition::Keep
    }

    fn draw_baselayer(&self) -> DrawBaselayer {
        DrawBaselayer::Custom
    }

    fn draw(&self, g: &mut GfxCtx, app: &App) {
        let mut opts = DrawOptions::new();
        opts.suppress_traffic_signal_details
            .extend(self.ic.members.clone());
        app.draw(g, opts, &ShowEverything::new());

        g.redraw(&self.draw);
    }
}
