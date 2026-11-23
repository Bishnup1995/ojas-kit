# l1 hidden acceptance tests for click.Duration (the agent never sees this file).
import datetime as dt
import pytest
import click
from click.testing import CliRunner
D = dt.timedelta

@pytest.mark.parametrize("s,exp", [("90s", D(seconds=90)), ("15m", D(minutes=15)), ("2h", D(hours=2)), ("1d", D(days=1)),
    ("1h30m", D(hours=1, minutes=30)), ("1d2h3m4s", D(days=1, hours=2, minutes=3, seconds=4)), ("45", D(seconds=45)),
    ("2H", D(hours=2)), ("0s", D(0)), ("  10m ", D(minutes=10))])
def test_convert(s, exp):
    assert click.Duration().convert(s, None, None) == exp

def test_timedelta_passthrough():
    assert click.Duration().convert(D(minutes=5), None, None) == D(minutes=5)

@pytest.mark.parametrize("bad", ["", "abc", "1x", "5m1h", "1h1h", "-5s", "1.5h", "h", "1 h"])
def test_invalid(bad):
    with pytest.raises(click.BadParameter) as e:
        click.Duration().convert(bad, None, None)
    assert "is not a valid duration" in str(e.value)

def test_range():
    t = click.Duration(min=D(seconds=10), max=D(hours=1))
    assert t.convert("10s", None, None) == D(seconds=10)
    assert t.convert("1h", None, None) == D(hours=1)
    for v in ("9s", "1h1s"):
        with pytest.raises(click.BadParameter) as e:
            t.convert(v, None, None)
        assert "is not in the range" in str(e.value)

def test_name_and_info():
    assert click.Duration.name == "duration"
    assert click.Duration().to_info_dict()["name"] == "duration"

def test_cli():
    @click.command()
    @click.option("--timeout", type=click.Duration(), default="30s")
    def cli(timeout):
        click.echo(int(timeout.total_seconds()))
    r = CliRunner()
    assert r.invoke(cli, []).output.strip() == "30"
    assert r.invoke(cli, ["--timeout", "1h2m"]).output.strip() == "3720"
    res = r.invoke(cli, ["--timeout", "soon"])
    assert res.exit_code == 2 and "is not a valid duration" in res.output
