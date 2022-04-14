FROM python:latest as builder
COPY dash-templates/main.py dash-templates/panel.json.template  dash-templates/dash.json.template accounts.json ./
RUN python main.py

FROM grafana/grafana:latest
COPY --from=builder out.json /usr/share/grafana/public/home.json
