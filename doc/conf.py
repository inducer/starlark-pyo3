from urllib.request import urlopen
import tomli

_conf_url = (
    "https://raw.githubusercontent.com/inducer/sphinxconfig/main/sphinxconfig.py"
)
with urlopen(_conf_url) as _inf:
    exec(compile(_inf.read(), _conf_url, "exec"), globals())

copyright = "2022, Andreas Kloeckner"

with open("../pyproject.toml", "rb") as f:
    release = tomli.load(f)["project"]["version"]

exclude_patterns = ["_build", "Thumbs.db", ".DS_Store"]

intersphinx_mapping = {
        "python": ("https://docs.python.org/dev", None),
}
