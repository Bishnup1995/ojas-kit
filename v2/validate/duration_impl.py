

class Duration(ParamType):
    """Parse ``1d2h3m4s``-style durations into :class:`datetime.timedelta`."""

    name = "duration"
    _re = __import__("re").compile(r"(?:(\d+)d)?(?:(\d+)h)?(?:(\d+)m)?(?:(\d+)s)?", __import__("re").I)

    def __init__(self, min=None, max=None):
        self.min = min
        self.max = max

    def convert(self, value, param, ctx):
        import datetime

        if isinstance(value, datetime.timedelta):
            td = value
        else:
            s = str(value).strip()
            if s.isdigit():
                td = datetime.timedelta(seconds=int(s))
            else:
                m = self._re.fullmatch(s)
                if not s or m is None or not any(m.groups()):
                    self.fail(f"{value!r} is not a valid duration.", param, ctx)
                d, h, mi, se = (int(x or 0) for x in m.groups())
                td = datetime.timedelta(days=d, hours=h, minutes=mi, seconds=se)
        if (self.min is not None and td < self.min) or (self.max is not None and td > self.max):
            self.fail(f"{td} is not in the range {self.min} to {self.max}.", param, ctx)
        return td
