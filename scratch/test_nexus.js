const apiKey = "D3dYUGpQOHUxSEV4ZklqRjFUSUtjVzR5VE11bk13c1dyUk5WckcvaUpwWmdzK0tqNTA4Z1BRPT0tLVh0M3A4aGpxSHRDRnUvU2stLUc3Uzd3RDBMNE83WlBmdzIraEplWFE9PQ=="; // API Key de prueba temporal

async function testEndpoint(url) {
  const query = `
    query GetModFiles($game: String!, $id: Int!) {
      mod(gameDomain: $game, modId: $id) {
        name
        files(view: ALL) {
          nodes {
            fileId
            name
            version
            categoryName
          }
        }
      }
    }
  `;

  console.log(`Testing URL: ${url}`);
  try {
    const response = await fetch(url, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "apikey": apiKey,
        "User-Agent": "PalModManager/1.0.0"
      },
      body: JSON.stringify({
        query,
        variables: { game: "palworld", id: 3787 }
      }),
    });

    console.log(`Status: ${response.status} ${response.statusText}`);
    const text = await response.text();
    console.log(`Response snippet: ${text.slice(0, 500)}`);
  } catch (err) {
    console.error(`Error: ${err.message}`);
  }
}

async function run() {
  await testEndpoint("https://graphql.nexusmods.com/");
  console.log("\n====================\n");
  await testEndpoint("https://api.nexusmods.com/v2/graphql");
}

run();
