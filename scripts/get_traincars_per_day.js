///@ts-check
const childProcess = require("child_process");
const fs = require("node:fs/promises");
const QUERY_FOR_DAY = (
  /** @type {Date} */ startDate,
  /** @type {Date} */ endDate,
) =>
  `
select distinct
  trainno,
  consist
from (
  select
    trainno,
    consist,
    received_at
  from
    records
  where
    received_at > '%START_DATE%'::timestamp  and received_at < '%END_DATE%'::timestamp
    and consist not like '%TDB%' and consist != ''
  order by
    trainno desc
)
order by trainno
;
`
    .replace("%START_DATE%", startDate.toISOString())
    .replace("%END_DATE%", endDate.toISOString());

/**
 * @param {string} command
 * */
async function spawnAndWait(command) {
  return new Promise((res, rej) => {
    childProcess.exec(command, (err, stdout, _stderr) => {
      if (err) {
        rej(err);
        return;
      }
      console.error(_stderr);
      res(stdout);
    });
  });
}

/**
 * @param {string} c
 * @param {number} min
 * @param {number} max
 */
function inRange(c, min, max) {
  let num = typeof c === "number" ? c : parseInt(c);
  if (isNaN(num)) {
    return false;
  }
  if (num >= min && num <= max) {
    return true;
  }
  return false;
}
(async function () {
  const DAYS = 1;
  const TF_HOURS = 24 * 60 * 60 * 1000;
  const day_cars = [];
  for (let i = 0; i < 18; i++) {
    const startDate = new Date(2025, 9, 10 + i, 4, 0, 0);
    const endDate = new Date(startDate.getTime() + DAYS * TF_HOURS);
    const query = QUERY_FOR_DAY(startDate, endDate);
    let sl1 = -1,
      sl2 = -1,
      sl3 = -1,
      sl4 = -1,
      sl5 = -1;
    console.log(startDate, "|", endDate);
    /** @type {string} */
    let res = await spawnAndWait(
      `echo "${query}" | psql --tuples-only historical_septa`,
    );
    let cars = new Set();
    res
      .split("\n")
      .filter((l) => l.length > 1)
      .forEach((l) => {
        let [trainno, $consist, ..._ee] = l.split("|");
        trainno = trainno.trim();
        let consist = $consist
          .trim()
          .split(",")
          .filter((c) => c.length > 1);
        consist.forEach((c) => {
          if (c.trim() === "TBD")
            return;
          cars.add(c);
        });
        return { trainno, consist };
      });

    [...cars.values()].forEach((c) => {
      if (inRange(c, 220, 239)) { sl3++; }
      /**
        * Right now there's SLIV:
          101–188, 306–399, 417–460
          (married pairs)
          276–305, 400–416
          (single cars)
          And for V:
          701–738 (single cars)
          801–882 (married pairs)
      */

      if (inRange(c, 101,   188)) { sl4++; }
      if (inRange(c, 306,   399)) { sl4++; }
      if (inRange(c, 417,   460)) { sl4++; }
      if (inRange(c, 276,   305)) { sl4++; }
      if (inRange(c, 400,   416)) { sl4++; }

      // if (inRange(c, 274,   303)) { sl4++; }
      // if (inRange(c, 9018, 9031)) { sl4++; }
      // if (inRange(c, 101,   188)) { sl4++; }

      if (inRange(c, 701, 738)) { sl5++; }
      if (inRange(c, 801, 882)) { sl5++; }
    });

    day_cars.push({
      date: startDate,
      endDate: endDate,
      dateString: startDate.toString(),
      cars: [...cars.values()],
      bok_breakdown: {
        sl1,
        sl2,
        sl3,
        sl4,
        sl5,
      },
    });
  }
  fs.writeFile("test.json", JSON.stringify({ data: day_cars }));
})();
